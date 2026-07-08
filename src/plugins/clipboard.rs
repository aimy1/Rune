use crate::core::plugin::{Context, ExecutionResult, Plugin, SearchResult};
use crate::search::Matcher;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub struct ClipboardPlugin {
    history: Vec<String>,
}

impl ClipboardPlugin {
    pub fn new(cache_dir: &Path) -> Self {
        let history = Self::load_history(&cache_dir.join("clipboard_history.json"));
        Self { history }
    }

    fn load_history(path: &Path) -> Vec<String> {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                return serde_json::from_str(&content).unwrap_or_default();
            }
        }
        Vec::new()
    }

    fn save_history(path: &Path, history: &[String]) {
        if let Ok(content) = serde_json::to_string_pretty(history) {
            let _ = fs::write(path, content);
        }
    }
}

pub fn run_clipboard_daemon(cache_dir: &Path) {
    let history_path = cache_dir.join("clipboard_history.json");
    let mut last_content = String::new();

    // Try loading initial clipboard text to seed it
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        if let Ok(text) = clipboard.get_text() {
            last_content = text.trim().to_string();
        }
    }

    loop {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            if let Ok(text) = clipboard.get_text() {
                let trimmed = text.trim().to_string();
                if !trimmed.is_empty() && trimmed != last_content {
                    last_content = trimmed.clone();

                    let mut history = ClipboardPlugin::load_history(&history_path);
                    // De-duplicate: move to top
                    history.retain(|item| item != &last_content);
                    history.insert(0, last_content.clone());
                    history.truncate(100); // Max 100 history items

                    ClipboardPlugin::save_history(&history_path, &history);
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
}

impl Plugin for ClipboardPlugin {
    fn id(&self) -> &'static str {
        "clipboard"
    }

    fn name(&self) -> &'static str {
        "Clipboard History"
    }

    fn description(&self) -> &'static str {
        "Search and restore clipboard records"
    }

    fn search(&self, query: &str, _cache_dir: &Path) -> Vec<SearchResult> {
        let mut results = Vec::new();
        let matcher = Matcher::new();

        for (idx, item) in self.history.iter().enumerate() {
            // Prepare preview representation of the clipboard text (limit single line)
            let one_line = item.replace('\n', " ").trim().to_string();
            let display_text = if one_line.len() > 60 {
                format!("{}...", &one_line[..60])
            } else {
                one_line
            };

            if query.is_empty() {
                let mut metadata = HashMap::new();
                metadata.insert("content".to_string(), item.clone());
                results.push(SearchResult {
                    id: format!("clip_{idx}"),
                    title: display_text,
                    subtitle: Some(format!("Len: {} chars", item.len())),
                    score: 0,
                    plugin_id: self.id(),
                    metadata,
                });
            } else if let Some(score) = matcher.fuzzy_match(item, query) {
                let mut metadata = HashMap::new();
                metadata.insert("content".to_string(), item.clone());
                results.push(SearchResult {
                    id: format!("clip_{idx}"),
                    title: display_text,
                    subtitle: Some(format!("Len: {} chars", item.len())),
                    score: score as i64,
                    plugin_id: self.id(),
                    metadata,
                });
            }
        }

        // Sort by search match score, or default index (descending order of copy) if score is 0
        results.sort_by(|a, b| b.score.cmp(&a.score));
        results
    }

    fn preview(&self, item: &SearchResult) -> Option<String> {
        let content = item.metadata.get("content")?;
        Some(format!(
            "# Clipboard Item\n\n```text\n{content}\n```\n\n*Press Enter to copy this text back to the clipboard.*",
        ))
    }

    fn execute(&self, item: &SearchResult, _ctx: &mut Context) -> ExecutionResult {
        if let Some(content) = item.metadata.get("content") {
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                if clipboard.set_text(content.clone()).is_ok() {
                    return ExecutionResult::Message("Restored to clipboard!".to_string());
                }
            }
            ExecutionResult::Message("Failed to restore clipboard".to_string())
        } else {
            ExecutionResult::Message("Clipboard content not found".to_string())
        }
    }
}
