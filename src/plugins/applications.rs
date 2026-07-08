use crate::core::plugin::{Context, ExecutionResult, Plugin, SearchResult};
use crate::search::Matcher;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub name: String,
    pub exec: String,
    pub icon: Option<String>,
    pub comment: Option<String>,
    pub file_path: String,
}

pub struct ApplicationsPlugin {
    apps: Arc<RwLock<Vec<AppInfo>>>,
}

impl ApplicationsPlugin {
    pub fn new(cache_dir: &Path) -> Self {
        let apps = Arc::new(RwLock::new(Vec::new()));
        let apps_clone = apps.clone();
        let cache_dir_buf = cache_dir.to_path_buf();

        // 1. Load cache immediately for sub-30ms startup
        if let Some(cached) = Self::load_cache(&cache_dir_buf) {
            if let Ok(mut guard) = apps.write() {
                *guard = cached;
            }
        }

        // 2. Spawn async task to refresh application index
        tokio::spawn(async move {
            let scanned = Self::scan_system_apps();
            if !scanned.is_empty() {
                Self::save_cache(&cache_dir_buf, &scanned);
                if let Ok(mut guard) = apps_clone.write() {
                    *guard = scanned;
                }
            }
        });

        Self { apps }
    }

    fn load_cache(cache_dir: &Path) -> Option<Vec<AppInfo>> {
        let path = cache_dir.join("applications.json");
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                return serde_json::from_str(&content).ok();
            }
        }
        None
    }

    fn save_cache(cache_dir: &Path, apps: &[AppInfo]) {
        let path = cache_dir.join("applications.json");
        if let Ok(content) = serde_json::to_string_pretty(apps) {
            let _ = fs::write(path, content);
        }
    }

    fn scan_system_apps() -> Vec<AppInfo> {
        let mut apps = Vec::new();
        let mut seen_execs = std::collections::HashSet::new();

        let search_dirs = vec![
            PathBuf::from("/usr/share/applications"),
            PathBuf::from("/usr/local/share/applications"),
            dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("/home/fd/.local/share"))
                .join("applications"),
        ];

        for dir in search_dirs {
            if !dir.exists() {
                continue;
            }
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                        if let Some(app) = Self::parse_desktop_file(&path) {
                            // De-duplicate applications by Exec command to keep list clean
                            if !seen_execs.contains(&app.exec) {
                                seen_execs.insert(app.exec.clone());
                                apps.push(app);
                            }
                        }
                    }
                }
            }
        }

        // Sort applications by name alphabetically
        apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        apps
    }

    fn parse_desktop_file(path: &Path) -> Option<AppInfo> {
        let content = fs::read_to_string(path).ok()?;
        let mut in_desktop_entry = false;
        let mut name = None;
        let mut exec = None;
        let mut icon = None;
        let mut comment = None;
        let mut no_display = false;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                in_desktop_entry = trimmed == "[Desktop Entry]";
                continue;
            }
            if !in_desktop_entry {
                continue;
            }

            if let Some(eq_idx) = trimmed.find('=') {
                let key = trimmed[..eq_idx].trim();
                let value = trimmed[eq_idx + 1..].trim();

                match key {
                    "Name" if name.is_none() => name = Some(value.to_string()),
                    "Exec" if exec.is_none() => exec = Some(value.to_string()),
                    "Icon" if icon.is_none() => icon = Some(value.to_string()),
                    "Comment" if comment.is_none() => comment = Some(value.to_string()),
                    "NoDisplay" => no_display = value.to_lowercase() == "true",
                    _ => {}
                }
            }
        }

        if no_display {
            return None;
        }

        let name = name?;
        let exec = exec?;

        // Clean Exec line from placeholders like %u, %F, %U
        let cleaned_exec = exec
            .split_whitespace()
            .filter(|part| !part.starts_with('%'))
            .collect::<Vec<&str>>()
            .join(" ");

        Some(AppInfo {
            name,
            exec: cleaned_exec,
            icon,
            comment,
            file_path: path.to_string_lossy().to_string(),
        })
    }
}

impl Plugin for ApplicationsPlugin {
    fn id(&self) -> &'static str {
        "applications"
    }

    fn name(&self) -> &'static str {
        "Applications"
    }

    fn description(&self) -> &'static str {
        "Launch system desktop applications"
    }

    fn search(&self, query: &str, _cache_dir: &Path) -> Vec<SearchResult> {
        let apps = match self.apps.read() {
            Ok(guard) => guard.clone(),
            Err(_) => return Vec::new(),
        };

        let matcher = Matcher::new();
        let mut results = Vec::new();

        for app in apps {
            let score_name = matcher.fuzzy_match(&app.name, query);
            let score_comment = app.comment.as_ref().and_then(|c| matcher.fuzzy_match(c, query));
            
            if let Some(score) = score_name.or(score_comment) {
                let mut metadata = HashMap::new();
                metadata.insert("exec".to_string(), app.exec.clone());
                if let Some(ref comment) = app.comment {
                    metadata.insert("comment".to_string(), comment.clone());
                }
                if let Some(ref icon) = app.icon {
                    metadata.insert("icon".to_string(), icon.clone());
                }

                results.push(SearchResult {
                    id: app.name.clone(),
                    title: app.name,
                    subtitle: Some(app.exec),
                    score: score as i64,
                    plugin_id: self.id(),
                    metadata,
                });
            }
        }

        results
    }

    fn preview(&self, item: &SearchResult) -> Option<String> {
        let mut preview = format!("# {}\n\n", item.title);
        if let Some(exec) = item.metadata.get("exec") {
            preview.push_str(&format!("**Exec**: `{}`\n", exec));
        }
        if let Some(comment) = item.metadata.get("comment") {
            preview.push_str(&format!("**Comment**: {}\n", comment));
        }
        if let Some(icon) = item.metadata.get("icon") {
            preview.push_str(&format!("**Icon**: {}\n", icon));
        }
        Some(preview)
    }

    fn execute(&self, item: &SearchResult, ctx: &mut Context) -> ExecutionResult {
        if let Some(exec) = item.metadata.get("exec") {
            ctx.run_command(exec.clone(), vec![], false);
            ExecutionResult::Exit
        } else {
            ExecutionResult::Message("Application executable path not found".to_string())
        }
    }
}
