use crate::core::plugin::{Context, ExecutionResult, Plugin, SearchResult};
use crate::search::Matcher;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

pub struct SystemdPlugin;

impl SystemdPlugin {
    pub fn new() -> Self {
        Self
    }

    fn get_services(&self) -> Vec<(String, String, String)> {
        // Runs systemctl to get services. Returns (name, active_state, description)
        let output = Command::new("systemctl")
            .args(["list-units", "--type=service", "--all", "--no-legend", "--no-pager"])
            .output();

        let mut services = Vec::new();
        if let Ok(out) = output {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                for line in stdout.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 4 {
                        let name = parts[0].to_string();
                        // parts[1]: load state (e.g. loaded)
                        // parts[2]: active state (e.g. active)
                        // parts[3]: sub state (e.g. running)
                        let active = format!("{} ({})", parts[2], parts[3]);
                        // Combine the description from the rest of the parts
                        let desc = parts[4..].join(" ");
                        services.push((name, active, desc));
                    }
                }
            }
        }
        services
    }
}

impl Plugin for SystemdPlugin {
    fn id(&self) -> &'static str {
        "systemd"
    }

    fn name(&self) -> &'static str {
        "Systemd"
    }

    fn description(&self) -> &'static str {
        "Manage systemd services"
    }

    fn search(&self, query: &str, _cache_dir: &Path) -> Vec<SearchResult> {
        let services = self.get_services();
        let matcher = Matcher::new();
        let mut results = Vec::new();

        for (name, active, desc) in services {
            let matches = if query.is_empty() {
                Some(0)
            } else {
                matcher.fuzzy_match(&name, query).or(matcher.fuzzy_match(&desc, query))
            };

            if let Some(score) = matches {
                let mut metadata = HashMap::new();
                metadata.insert("name".to_string(), name.clone());
                metadata.insert("active".to_string(), active.clone());
                metadata.insert("description".to_string(), desc.clone());

                results.push(SearchResult {
                    id: format!("systemd_{name}"),
                    title: name,
                    subtitle: Some(active),
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
        let active = item.metadata.get("active")?;
        let desc = item.metadata.get("description")?;

        let mut preview = format!(
            "# Systemd Service\n\n**Name**: `{name}`\n**State**: `{active}`\n**Description**: {desc}\n\n",
        );

        // Try getting systemctl status logs
        let out = Command::new("systemctl")
            .args(["status", "--no-pager", "-n", "20", name])
            .output();

        preview.push_str("### Service Status Logs:\n");
        if let Ok(o) = out {
            let logs = String::from_utf8_lossy(&o.stdout);
            let err_logs = String::from_utf8_lossy(&o.stderr);
            preview.push_str("```text\n");
            if !logs.trim().is_empty() {
                preview.push_str(&logs);
            }
            if !err_logs.trim().is_empty() {
                preview.push_str(&err_logs);
            }
            preview.push_str("\n```\n");
        } else {
            preview.push_str("*Could not fetch service status logs.*\n");
        }

        preview.push_str("\n*Press Enter to restart this service (may prompt for password).*");
        Some(preview)
    }

    fn execute(&self, item: &SearchResult, ctx: &mut Context) -> ExecutionResult {
        let name = match item.metadata.get("name") {
            Some(n) => n,
            None => return ExecutionResult::Message("Service name not found".to_string()),
        };

        // Run systemctl restart. We run it in terminal so polkit password prompt shows up cleanly
        ctx.run_command("systemctl".to_string(), vec!["restart".to_string(), name.clone()], true);
        ExecutionResult::Exit
    }
}
