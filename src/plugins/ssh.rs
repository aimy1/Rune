use crate::core::plugin::{Context, ExecutionResult, Plugin, SearchResult};
use crate::search::Matcher;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct SshPlugin {
    hosts: Vec<String>,
}

impl SshPlugin {
    pub fn new() -> Self {
        let hosts = Self::parse_ssh_config();
        Self { hosts }
    }

    fn parse_ssh_config() -> Vec<String> {
        let mut hosts = Vec::new();
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home/fd"));
        let ssh_config = home.join(".ssh/config");

        if ssh_config.exists() {
            if let Ok(content) = fs::read_to_string(ssh_config) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    // Skip comments
                    if trimmed.starts_with('#') {
                        continue;
                    }
                    if trimmed.to_lowercase().starts_with("host ") {
                        // Extract host alias(es)
                        let hosts_part = &trimmed[5..].trim();
                        // Can be space-separated multiples
                        for host in hosts_part.split_whitespace() {
                            if host != "*" && !host.contains('?') && !host.contains('!') {
                                hosts.push(host.to_string());
                            }
                        }
                    }
                }
            }
        }
        hosts
    }
}

impl Plugin for SshPlugin {
    fn id(&self) -> &'static str {
        "ssh"
    }

    fn name(&self) -> &'static str {
        "SSH Launcher"
    }

    fn description(&self) -> &'static str {
        "Connect to configured SSH hosts"
    }

    fn search(&self, query: &str, _cache_dir: &Path) -> Vec<SearchResult> {
        let mut results = Vec::new();
        let matcher = Matcher::new();

        for host in &self.hosts {
            if query.is_empty() {
                let mut metadata = HashMap::new();
                metadata.insert("host".to_string(), host.clone());
                results.push(SearchResult {
                    id: format!("ssh_{host}"),
                    title: host.clone(),
                    subtitle: Some("SSH host".to_string()),
                    score: 0,
                    plugin_id: self.id(),
                    metadata,
                });
            } else if let Some(score) = matcher.fuzzy_match(host, query) {
                let mut metadata = HashMap::new();
                metadata.insert("host".to_string(), host.clone());
                results.push(SearchResult {
                    id: format!("ssh_{host}"),
                    title: host.clone(),
                    subtitle: Some("SSH host".to_string()),
                    score: score as i64,
                    plugin_id: self.id(),
                    metadata,
                });
            }
        }

        results.sort_by(|a, b| b.score.cmp(&a.score));
        results
    }

    fn preview(&self, item: &SearchResult) -> Option<String> {
        let host = item.metadata.get("host")?;
        Some(format!(
            "# SSH Connection\n\nHost Alias: `{host}`\n\nCommand:\n```bash\nssh {host}\n```\n\n*Press Enter to start this SSH connection in your terminal.*",
        ))
    }

    fn execute(&self, item: &SearchResult, ctx: &mut Context) -> ExecutionResult {
        if let Some(host) = item.metadata.get("host") {
            ctx.run_command("ssh".to_string(), vec![host.clone()], true); // run in terminal
            ExecutionResult::Exit
        } else {
            ExecutionResult::Message("SSH Host not found".to_string())
        }
    }
}
