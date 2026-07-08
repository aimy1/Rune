use crate::core::plugin::{Context, ExecutionResult, Plugin, SearchResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Serialize, Deserialize)]
pub struct ExternalItem {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub score: i64,
    pub preview: Option<String>,
    pub execute_cmd: Option<String>,
    pub execute_args: Option<Vec<String>>,
    pub run_in_terminal: Option<bool>,
}

pub struct ExternalPlugin {
    path: PathBuf,
    name: String,
    id: String,
}

impl ExternalPlugin {
    pub fn new(path: PathBuf) -> Self {
        let filename = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "external".to_string());

        let name = filename.clone();
        let id = format!("external_{filename}");

        Self { path, name, id }
    }

    pub fn scan_external_plugins(plugins_dir: &Path) -> Vec<Self> {
        let mut plugins = Vec::new();
        if !plugins_dir.exists() {
            return plugins;
        }

        if let Ok(entries) = fs::read_dir(plugins_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    // Check if file is executable
                    if let Ok(metadata) = path.metadata() {
                        let permissions = metadata.permissions();
                        // On Unix, check if executable bit is set
                        if permissions.mode() & 0o111 != 0 {
                            plugins.push(ExternalPlugin::new(path));
                        }
                    }
                }
            }
        }

        plugins
    }
}

impl Plugin for ExternalPlugin {
    fn id(&self) -> &'static str {
        // Safe leak because plugin list is constructed once on startup
        Box::leak(self.id.clone().into_boxed_str())
    }

    fn name(&self) -> &'static str {
        Box::leak(self.name.clone().into_boxed_str())
    }

    fn description(&self) -> &'static str {
        "External script plugin"
    }

    fn search(&self, query: &str, _cache_dir: &Path) -> Vec<SearchResult> {
        let output = Command::new(&self.path)
            .arg(query)
            .output();

        let mut results = Vec::new();

        if let Ok(out) = output {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                if let Ok(items) = serde_json::from_str::<Vec<ExternalItem>>(&stdout) {
                    for item in items {
                        let mut metadata = HashMap::new();
                        if let Some(ref p) = item.preview {
                            metadata.insert("preview".to_string(), p.clone());
                        }
                        if let Some(ref cmd) = item.execute_cmd {
                            metadata.insert("execute_cmd".to_string(), cmd.clone());
                        }
                        if let Some(ref args) = item.execute_args {
                            if let Ok(args_str) = serde_json::to_string(args) {
                                metadata.insert("execute_args".to_string(), args_str);
                            }
                        }
                        if let Some(run_term) = item.run_in_terminal {
                            metadata.insert("run_in_terminal".to_string(), run_term.to_string());
                        }

                        results.push(SearchResult {
                            id: item.id,
                            title: item.title,
                            subtitle: item.subtitle,
                            score: item.score,
                            plugin_id: self.id(),
                            metadata,
                        });
                    }
                }
            }
        }

        results
    }

    fn preview(&self, item: &SearchResult) -> Option<String> {
        item.metadata.get("preview").cloned()
    }

    fn execute(&self, item: &SearchResult, ctx: &mut Context) -> ExecutionResult {
        let cmd = match item.metadata.get("execute_cmd") {
            Some(c) => c.clone(),
            None => return ExecutionResult::Success,
        };

        let args: Vec<String> = item
            .metadata
            .get("execute_args")
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        let run_in_terminal = item
            .metadata
            .get("run_in_terminal")
            .and_then(|s| s.parse::<bool>().ok())
            .unwrap_or(false);

        ctx.run_command(cmd, args, run_in_terminal);
        ExecutionResult::Exit
    }
}
