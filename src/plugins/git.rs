use crate::core::plugin::{Context, ExecutionResult, Plugin, SearchResult};
use crate::search::Matcher;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

pub struct GitPlugin;

impl GitPlugin {
    pub fn new() -> Self {
        Self
    }

    fn is_git_repo(&self) -> bool {
        let output = Command::new("git")
            .args(["rev-parse", "--is-inside-work-tree"])
            .output();
        if let Ok(out) = output {
            out.status.success() && String::from_utf8_lossy(&out.stdout).trim() == "true"
        } else {
            false
        }
    }

    fn get_branches(&self) -> Vec<String> {
        let output = Command::new("git")
            .args(["branch", "--format=%(refname:short)"])
            .output();
        if let Ok(out) = output {
            if out.status.success() {
                return String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }
        Vec::new()
    }
}

impl Plugin for GitPlugin {
    fn id(&self) -> &'static str {
        "git"
    }

    fn name(&self) -> &'static str {
        "Git"
    }

    fn description(&self) -> &'static str {
        "Common Git repository actions"
    }

    fn search(&self, query: &str, _cache_dir: &Path) -> Vec<SearchResult> {
        if !self.is_git_repo() {
            return Vec::new(); // Silent when not in git repository
        }

        let mut items: Vec<(String, String, String, String)> = Vec::new();

        // 1. Built-in git actions
        items.push((
            "status".to_string(),
            "git status".to_string(),
            "View repository status".to_string(),
            "status".to_string(),
        ));
        items.push((
            "pull".to_string(),
            "git pull".to_string(),
            "Fetch and integrate with local branch".to_string(),
            "pull".to_string(),
        ));
        items.push((
            "push".to_string(),
            "git push".to_string(),
            "Update remote refs and branches".to_string(),
            "push".to_string(),
        ));
        items.push((
            "log".to_string(),
            "git log --oneline -n 15".to_string(),
            "Show commit history summary".to_string(),
            "log".to_string(),
        ));

        // 2. Add branch switching actions
        for branch in self.get_branches() {
            items.push((
                "checkout".to_string(),
                format!("git checkout {branch}"),
                format!("Switch active branch to '{branch}'"),
                format!("checkout_{branch}"),
            ));
        }

        let matcher = Matcher::new();
        let mut results = Vec::new();

        for (action, title, desc, id_suffix) in items {
            let matches = if query.is_empty() {
                Some(0)
            } else {
                matcher.fuzzy_match(&title, query).or(matcher.fuzzy_match(&desc, query))
            };

            if let Some(score) = matches {
                let mut metadata = HashMap::new();
                metadata.insert("action".to_string(), action);
                metadata.insert("command".to_string(), title.clone());

                results.push(SearchResult {
                    id: format!("git_{id_suffix}"),
                    title,
                    subtitle: Some(desc),
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
        let action = item.metadata.get("action")?;
        let cmd = item.metadata.get("command")?;

        let mut preview = format!("# Git Action: {}\n\nCommand:\n```bash\n{cmd}\n```\n\n", action);

        match action.as_str() {
            "status" => {
                let out = Command::new("git").args(["status", "-s"]).output().ok();
                let status_text = out
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                    .unwrap_or_default();
                preview.push_str("### Current Status Summary:\n");
                if status_text.is_empty() {
                    preview.push_str("*Working directory clean.*\n");
                } else {
                    preview.push_str(&format!("```text\n{status_text}\n```\n"));
                }
            }
            "log" => {
                let out = Command::new("git")
                    .args(["log", "--oneline", "-n", "10"])
                    .output()
                    .ok();
                let log_text = out
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                    .unwrap_or_default();
                preview.push_str("### Recent Commit Log:\n");
                preview.push_str(&format!("```text\n{log_text}\n```\n"));
            }
            "checkout" => {
                preview.push_str("Runs checkout in terminal to switch active branch.\n");
            }
            _ => {
                preview.push_str("Runs the command in your terminal.\n");
            }
        }

        preview.push_str("\n*Press Enter to run this git action in terminal.*");
        Some(preview)
    }

    fn execute(&self, item: &SearchResult, ctx: &mut Context) -> ExecutionResult {
        if let Some(cmd) = item.metadata.get("command") {
            ctx.run_command(cmd.clone(), vec![], true);
            ExecutionResult::Exit
        } else {
            ExecutionResult::Message("Git command not found".to_string())
        }
    }
}
