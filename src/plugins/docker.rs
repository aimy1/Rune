use crate::core::plugin::{Context, ExecutionResult, Plugin, SearchResult};
use crate::search::Matcher;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

pub struct DockerPlugin;

impl DockerPlugin {
    pub fn new() -> Self {
        Self
    }

    fn check_docker(&self) -> bool {
        let output = Command::new("docker").arg("info").output();
        if let Ok(out) = output {
            out.status.success()
        } else {
            false
        }
    }

    fn get_containers(&self) -> Vec<(String, String, String)> {
        // Returns Vec of (id, name, status)
        let output = Command::new("docker")
            .args(["ps", "-a", "--format", "{{.ID}}\t{{.Names}}\t{{.Status}}"])
            .output();

        let mut containers = Vec::new();
        if let Ok(out) = output {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                for line in stdout.lines() {
                    let parts: Vec<&str> = line.split('\t').collect();
                    if parts.len() >= 3 {
                        containers.push((
                            parts[0].to_string(),
                            parts[1].to_string(),
                            parts[2].to_string(),
                        ));
                    }
                }
            }
        }
        containers
    }
}

impl Plugin for DockerPlugin {
    fn id(&self) -> &'static str {
        "docker"
    }

    fn name(&self) -> &'static str {
        "Docker"
    }

    fn description(&self) -> &'static str {
        "Manage Docker containers"
    }

    fn search(&self, query: &str, _cache_dir: &Path) -> Vec<SearchResult> {
        if !self.check_docker() {
            return vec![SearchResult {
                id: "docker_inactive".to_string(),
                title: "Docker daemon not running".to_string(),
                subtitle: Some("Please start docker service first".to_string()),
                score: 0,
                plugin_id: self.id(),
                metadata: HashMap::new(),
            }];
        }

        let containers = self.get_containers();
        let matcher = Matcher::new();
        let mut results = Vec::new();

        for (id, name, status) in containers {
            let matches = if query.is_empty() {
                Some(0)
            } else {
                matcher.fuzzy_match(&name, query).or(matcher.fuzzy_match(&id, query))
            };

            if let Some(score) = matches {
                let mut metadata = HashMap::new();
                metadata.insert("id".to_string(), id.clone());
                metadata.insert("name".to_string(), name.clone());
                metadata.insert("status".to_string(), status.clone());

                results.push(SearchResult {
                    id: format!("docker_{id}"),
                    title: name,
                    subtitle: Some(format!("{id} | {status}")),
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
        let name = item.metadata.get("name")?;
        let id = item.metadata.get("id")?;
        let status = item.metadata.get("status")?;

        let mut preview = format!(
            "# Docker Container\n\n**Name**: `{name}`\n**ID**: `{id}`\n**Status**: `{status}`\n\n",
        );

        // Try getting container logs for preview
        let out = Command::new("docker")
            .args(["logs", "--tail", "25", id])
            .output()
            .ok();

        preview.push_str("### Recent Logs (Stderr & Stdout):\n");
        if let Some(o) = out {
            let logs = String::from_utf8_lossy(&o.stdout);
            let err_logs = String::from_utf8_lossy(&o.stderr);
            if logs.trim().is_empty() && err_logs.trim().is_empty() {
                preview.push_str("*No logs found.*\n");
            } else {
                preview.push_str("```text\n");
                if !logs.trim().is_empty() {
                    preview.push_str(&logs);
                }
                if !err_logs.trim().is_empty() {
                    preview.push_str(&err_logs);
                }
                preview.push_str("\n```\n");
            }
        } else {
            preview.push_str("*Could not fetch logs.*\n");
        }

        preview.push_str("\n*Press Enter to restart this container in the background.*");
        Some(preview)
    }

    fn execute(&self, item: &SearchResult, _ctx: &mut Context) -> ExecutionResult {
        if item.id == "docker_inactive" {
            return ExecutionResult::Success;
        }

        let name = match item.metadata.get("name") {
            Some(n) => n,
            None => return ExecutionResult::Message("Container name not found".to_string()),
        };

        // Default enter action: restart container
        let output = Command::new("docker")
            .args(["restart", name])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                ExecutionResult::Message(format!("Container '{name}' restarted successfully!"))
            }
            Ok(out) => {
                let err = String::from_utf8_lossy(&out.stderr);
                ExecutionResult::Message(format!("Failed to restart container: {err}"))
            }
            Err(e) => ExecutionResult::Message(format!("Error executing docker restart: {e}")),
        }
    }
}
