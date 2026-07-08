use crate::core::plugin::{Context, ExecutionResult, Plugin, SearchResult};
use crate::search::Matcher;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use walkdir::WalkDir;

pub struct FilesPlugin {
    files: Arc<RwLock<Vec<String>>>,
    _paths: Vec<String>,
    _ignore: Vec<String>,
    _max_depth: usize,
}

impl FilesPlugin {
    pub fn new(
        cache_dir: &Path,
        paths: Vec<String>,
        ignore: Vec<String>,
        max_depth: usize,
    ) -> Self {
        let files = Arc::new(RwLock::new(Vec::new()));
        let files_clone = files.clone();
        let cache_dir_buf = cache_dir.to_path_buf();

        let paths_resolved: Vec<PathBuf> = paths
            .iter()
            .map(|p| {
                if p == "~" {
                    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home/fd"))
                } else if p.starts_with("~/") {
                    dirs::home_dir()
                        .unwrap_or_else(|| PathBuf::from("/home/fd"))
                        .join(&p[2..])
                } else {
                    PathBuf::from(p)
                }
            })
            .collect();

        // 1. Load cached files list immediately
        if let Some(cached) = Self::load_cache(&cache_dir_buf) {
            if let Ok(mut guard) = files.write() {
                *guard = cached;
            }
        }

        // 2. Spawn indexing worker
        let ignore_clone = ignore.clone();
        tokio::spawn(async move {
            let scanned = Self::scan_files(&paths_resolved, &ignore_clone, max_depth);
            if !scanned.is_empty() {
                Self::save_cache(&cache_dir_buf, &scanned);
                if let Ok(mut guard) = files_clone.write() {
                    *guard = scanned;
                }
            }
        });

        Self {
            files,
            _paths: paths,
            _ignore: ignore,
            _max_depth: max_depth,
        }
    }

    fn load_cache(cache_dir: &Path) -> Option<Vec<String>> {
        let path = cache_dir.join("files.txt");
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                return Some(
                    content
                        .lines()
                        .map(|s| s.to_string())
                        .filter(|s| !s.is_empty())
                        .collect(),
                );
            }
        }
        None
    }

    fn save_cache(cache_dir: &Path, files: &[String]) {
        let path = cache_dir.join("files.txt");
        let content = files.join("\n");
        let _ = fs::write(path, content);
    }

    fn scan_files(dirs: &[PathBuf], ignore: &[String], max_depth: usize) -> Vec<String> {
        let mut files = Vec::new();
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home/fd"));

        for dir in dirs {
            if !dir.exists() {
                continue;
            }

            let walker = WalkDir::new(dir)
                .max_depth(max_depth)
                .into_iter()
                .filter_entry(|e| {
                    let file_name = e.file_name().to_string_lossy();
                    // Skip hidden files/directories except current directory
                    if file_name.starts_with('.') && file_name.len() > 1 {
                        return false;
                    }
                    // Skip specified ignore items
                    !ignore.iter().any(|ig| file_name == ig.as_str())
                });

            for entry in walker.flatten() {
                let path = entry.path();
                // Store path. Try making it relative to Home for clean display if possible
                let path_str = if path.starts_with(&home_dir) {
                    format!("~/{}", path.strip_prefix(&home_dir).unwrap().to_string_lossy())
                } else {
                    path.to_string_lossy().to_string()
                };
                files.push(path_str);
            }
        }

        // Limit search pool size to 50k items for memory/speed safety
        files.truncate(50000);
        files
    }

    fn resolve_path(&self, display_path: &str) -> PathBuf {
        if display_path == "~" {
            dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home/fd"))
        } else if display_path.starts_with("~/") {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/home/fd"))
                .join(&display_path[2..])
        } else {
            PathBuf::from(display_path)
        }
    }
}

impl Plugin for FilesPlugin {
    fn id(&self) -> &'static str {
        "files"
    }

    fn name(&self) -> &'static str {
        "Files"
    }

    fn description(&self) -> &'static str {
        "Search files and folders"
    }

    fn search(&self, query: &str, _cache_dir: &Path) -> Vec<SearchResult> {
        let files = match self.files.read() {
            Ok(guard) => guard.clone(),
            Err(_) => return Vec::new(),
        };

        if query.is_empty() {
            // Return top 20 items of index
            return files
                .into_iter()
                .take(20)
                .map(|f| {
                    let mut metadata = HashMap::new();
                    metadata.insert("path".to_string(), f.clone());
                    SearchResult {
                        id: f.clone(),
                        title: Path::new(&f)
                            .file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| f.clone()),
                        subtitle: Some(f),
                        score: 0,
                        plugin_id: self.id(),
                        metadata,
                    }
                })
                .collect();
        }

        let matcher = Matcher::new();
        let mut results = Vec::new();

        for f in files {
            let filename = Path::new(&f)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| f.clone());

            // Match on filename (gives higher score) or full path
            let score = if let Some(fn_score) = matcher.fuzzy_match(&filename, query) {
                Some(fn_score + 20) // boost filename matches
            } else {
                matcher.fuzzy_match(&f, query)
            };

            if let Some(score) = score {
                let mut metadata = HashMap::new();
                metadata.insert("path".to_string(), f.clone());
                results.push(SearchResult {
                    id: f.clone(),
                    title: filename,
                    subtitle: Some(f),
                    score: score as i64,
                    plugin_id: self.id(),
                    metadata,
                });
            }
        }

        // Sort by score descending
        results.sort_by(|a, b| b.score.cmp(&a.score));
        results.truncate(100); // Only return top 100 matches
        results
    }

    fn preview(&self, item: &SearchResult) -> Option<String> {
        let display_path = item.metadata.get("path")?;
        let real_path = self.resolve_path(display_path);

        if !real_path.exists() {
            return Some(format!("Path does not exist: {display_path}"));
        }

        if real_path.is_dir() {
            let mut preview = format!("# Directory: {display_path}\n\nContents:\n");
            if let Ok(entries) = fs::read_dir(&real_path) {
                let mut count = 0;
                for entry in entries.flatten() {
                    if count >= 20 {
                        preview.push_str("  ...\n");
                        break;
                    }
                    let name = entry.file_name().to_string_lossy().to_string();
                    let icon = if entry.path().is_dir() { "📁" } else { "📄" };
                    preview.push_str(&format!("  {icon} {name}\n"));
                    count += 1;
                }
            }
            Some(preview)
        } else {
            // Try to read it as a text file
            match fs::read_to_string(&real_path) {
                Ok(content) => {
                    let lines: Vec<&str> = content.lines().take(30).collect();
                    let file_size = fs::metadata(&real_path).map(|m| m.len()).unwrap_or(0);
                    let mut preview = format!(
                        "# File: {display_path}\n*Size: {file_size} bytes*\n\n```\n",
                    );
                    preview.push_str(&lines.join("\n"));
                    preview.push_str("\n```");
                    Some(preview)
                }
                Err(_) => {
                    // Probably binary
                    let meta = fs::metadata(&real_path).ok();
                    let size = meta.map(|m| m.len()).unwrap_or(0);
                    Some(format!(
                        "# Binary File: {display_path}\n*Size: {size} bytes*\n\n(Cannot render preview for binary file)",
                    ))
                }
            }
        }
    }

    fn execute(&self, item: &SearchResult, ctx: &mut Context) -> ExecutionResult {
        let display_path = match item.metadata.get("path") {
            Some(p) => p,
            None => return ExecutionResult::Message("File path metadata not found".to_string()),
        };
        let real_path = self.resolve_path(display_path);
        let path_str = real_path.to_string_lossy().to_string();

        ctx.run_command("xdg-open".to_string(), vec![path_str], false);
        ExecutionResult::Exit
    }
}
