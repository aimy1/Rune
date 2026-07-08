use crate::core::plugin::{Context, ExecutionResult, Plugin, SearchResult};
use crate::search::Matcher;
use std::collections::HashMap;
use std::path::Path;

pub struct CommandsPlugin;

impl CommandsPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for CommandsPlugin {
    fn id(&self) -> &'static str {
        "commands"
    }

    fn name(&self) -> &'static str {
        "Commands"
    }

    fn description(&self) -> &'static str {
        "Run shell commands in the terminal"
    }

    fn search(&self, query: &str, _cache_dir: &Path) -> Vec<SearchResult> {
        let mut results = Vec::new();

        // 1. Built-in command suggestions when query is short or empty
        let builtins = vec![
            ("htop", "Monitor system resources and processes"),
            ("df -h", "Show disk space usage in human-readable format"),
            ("free -m", "Show RAM memory usage in Megabytes"),
            ("ip a", "Display IP addresses and network interfaces"),
            ("journalctl -xe", "View systemd logs for failures"),
            ("ping -c 4 google.com", "Check network latency to google.com"),
        ];

        let matcher = Matcher::new();

        for (cmd, desc) in builtins {
            if query.is_empty() {
                let mut metadata = HashMap::new();
                metadata.insert("command".to_string(), cmd.to_string());
                results.push(SearchResult {
                    id: format!("builtin_{cmd}"),
                    title: cmd.to_string(),
                    subtitle: Some(desc.to_string()),
                    score: 0,
                    plugin_id: self.id(),
                    metadata,
                });
            } else if let Some(score) = matcher.fuzzy_match(cmd, query) {
                let mut metadata = HashMap::new();
                metadata.insert("command".to_string(), cmd.to_string());
                results.push(SearchResult {
                    id: format!("builtin_{cmd}"),
                    title: cmd.to_string(),
                    subtitle: Some(desc.to_string()),
                    score: score as i64,
                    plugin_id: self.id(),
                    metadata,
                });
            }
        }

        // 2. Add raw command execution option if user typed something
        if !query.is_empty() {
            let mut metadata = HashMap::new();
            metadata.insert("command".to_string(), query.to_string());
            results.push(SearchResult {
                id: "raw_cmd".to_string(),
                title: format!("Run: {query}"),
                subtitle: Some("Execute in terminal".to_string()),
                // Give it a high score so it shows up at the top if it matches the prefix or stands out
                score: 9999, 
                plugin_id: self.id(),
                metadata,
            });
        }

        // Sort by score descending
        results.sort_by(|a, b| b.score.cmp(&a.score));
        results
    }

    fn preview(&self, item: &SearchResult) -> Option<String> {
        let cmd = item.metadata.get("command")?;
        Some(format!(
            "# Command Execution\n\nCommand to run:\n```bash\n{cmd}\n```\n\n*Press Enter to run this command in your terminal.*",
        ))
    }

    fn execute(&self, item: &SearchResult, ctx: &mut Context) -> ExecutionResult {
        if let Some(cmd) = item.metadata.get("command") {
            ctx.run_command(cmd.clone(), vec![], true); // run in terminal
            ExecutionResult::Exit
        } else {
            ExecutionResult::Message("Command string not found".to_string())
        }
    }
}
