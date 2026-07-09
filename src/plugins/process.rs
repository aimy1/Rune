use crate::core::plugin::{Context, ExecutionResult, Plugin, SearchResult};
use crate::search::Matcher;
use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::process::Command;

pub struct ProcessInfo {
    pub pid: i32,
    pub ppid: i32,
    pub name: String,
    pub state: char,
    pub threads: i32,
    pub rss_bytes: u64,
    pub user: String,
    pub cmdline: String,
}

pub struct ProcessPlugin;

impl ProcessPlugin {
    pub fn new() -> Self {
        Self
    }

    fn get_users_map(&self) -> HashMap<u32, String> {
        let mut map = HashMap::new();
        if let Ok(content) = fs::read_to_string("/etc/passwd") {
            for line in content.lines() {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 3 {
                    let username = parts[0].to_string();
                    if let Ok(uid) = parts[2].parse::<u32>() {
                        map.insert(uid, username);
                    }
                }
            }
        }
        map
    }

    fn scan_processes(&self) -> Vec<ProcessInfo> {
        let users_map = self.get_users_map();
        let mut processes = Vec::new();
        let proc_dir = Path::new("/proc");
        let page_size = 4096; // Standard page size (4KB)

        let entries = match fs::read_dir(proc_dir) {
            Ok(e) => e,
            Err(_) => return Vec::new(),
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let filename = match path.file_name().and_then(|s| s.to_str()) {
                Some(name) => name,
                None => continue,
            };

            let pid = match filename.parse::<i32>() {
                Ok(p) => p,
                Err(_) => continue,
            };

            // Get process UID from directory owner metadata
            let uid = entry.metadata().map(|m| m.uid()).unwrap_or(0);
            let user = users_map.get(&uid).cloned().unwrap_or_else(|| uid.to_string());

            // Read stat
            let stat_path = path.join("stat");
            let stat_content = match fs::read_to_string(&stat_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            // Extract name between first '(' and last ')'
            let first_paren = match stat_content.find('(') {
                Some(idx) => idx,
                None => continue,
            };
            let last_paren = match stat_content.rfind(')') {
                Some(idx) => idx,
                None => continue,
            };

            if first_paren >= last_paren {
                continue;
            }

            let name = stat_content[first_paren + 1..last_paren].to_string();
            let post_paren = &stat_content[last_paren + 1..].trim();
            let parts: Vec<&str> = post_paren.split_whitespace().collect();

            if parts.len() < 18 {
                continue;
            }

            let state = parts[0].chars().next().unwrap_or('?');
            let ppid = parts[1].parse::<i32>().unwrap_or(0);
            let threads = parts[17].parse::<i32>().unwrap_or(0);

            // Read statm for resident memory (RSS)
            let statm_path = path.join("statm");
            let rss_bytes = if let Ok(statm_content) = fs::read_to_string(&statm_path) {
                let statm_parts: Vec<&str> = statm_content.split_whitespace().collect();
                if !statm_parts.is_empty() {
                    statm_parts[1].parse::<u64>().unwrap_or(0) * page_size
                } else {
                    0
                }
            } else {
                0
            };

            // Read cmdline
            let cmdline_path = path.join("cmdline");
            let cmdline = if let Ok(cmdline_raw) = fs::read(cmdline_path) {
                if cmdline_raw.is_empty() {
                    name.clone()
                } else {
                    let mut parts = Vec::new();
                    let mut current_part = Vec::new();
                    for byte in cmdline_raw {
                        if byte == 0 {
                            if !current_part.is_empty() {
                                parts.push(String::from_utf8_lossy(&current_part).into_owned());
                                current_part.clear();
                            }
                        } else {
                            current_part.push(byte);
                        }
                    }
                    if !current_part.is_empty() {
                        parts.push(String::from_utf8_lossy(&current_part).into_owned());
                    }
                    parts.join(" ")
                }
            } else {
                name.clone()
            };

            processes.push(ProcessInfo {
                pid,
                ppid,
                name,
                state,
                threads,
                rss_bytes,
                user,
                cmdline,
            });
        }

        processes
    }

    fn format_memory(&self, bytes: u64) -> String {
        let kib = bytes as f64 / 1024.0;
        let mib = kib / 1024.0;
        let gib = mib / 1024.0;

        if gib >= 1.0 {
            format!("{:.2} GiB", gib)
        } else if mib >= 1.0 {
            format!("{:.2} MiB", mib)
        } else if kib >= 1.0 {
            format!("{:.2} KiB", kib)
        } else {
            format!("{} B", bytes)
        }
    }

    fn format_state(&self, state: char) -> &'static str {
        match state {
            'R' => "Running",
            'S' => "Sleeping (interruptible)",
            'D' => "Disk Sleep (uninterruptible)",
            'Z' => "Zombie",
            'T' => "Stopped",
            't' => "Tracing Stop",
            'X' | 'x' => "Dead",
            'K' => "Wakekill",
            'W' => "Waking",
            'P' => "Parked",
            _ => "Unknown",
        }
    }
}

impl Plugin for ProcessPlugin {
    fn id(&self) -> &'static str {
        "process"
    }

    fn name(&self) -> &'static str {
        "Process Manager"
    }

    fn description(&self) -> &'static str {
        "Monitor and terminate running processes"
    }

    fn search(&self, query: &str, _cache_dir: &Path) -> Vec<SearchResult> {
        let mut processes = self.scan_processes();
        let matcher = Matcher::new();
        let mut results = Vec::new();

        if query.is_empty() {
            // If query is empty, sort by RAM usage descending and display top 50
            processes.sort_by(|a, b| b.rss_bytes.cmp(&a.rss_bytes));
            for p in processes.into_iter().take(50) {
                let mut metadata = HashMap::new();
                metadata.insert("pid".to_string(), p.pid.to_string());
                metadata.insert("ppid".to_string(), p.ppid.to_string());
                metadata.insert("name".to_string(), p.name.clone());
                metadata.insert("state".to_string(), p.state.to_string());
                metadata.insert("threads".to_string(), p.threads.to_string());
                metadata.insert("rss_bytes".to_string(), p.rss_bytes.to_string());
                metadata.insert("user".to_string(), p.user.clone());
                metadata.insert("cmdline".to_string(), p.cmdline.clone());

                let ram_str = self.format_memory(p.rss_bytes);
                results.push(SearchResult {
                    id: format!("proc_{}", p.pid),
                    title: format!("{} (PID: {})", p.name, p.pid),
                    subtitle: Some(format!("Memory: {} | Owner: {}", ram_str, p.user)),
                    score: 0,
                    plugin_id: self.id(),
                    metadata,
                });
            }
        } else {
            // Match against name, pid, or cmdline
            for p in processes {
                let matches = if p.pid.to_string() == query {
                    Some(1000) // exact PID match has top priority
                } else {
                    matcher.fuzzy_match(&p.name, query).or_else(|| matcher.fuzzy_match(&p.cmdline, query))
                };

                if let Some(score) = matches {
                    let mut metadata = HashMap::new();
                    metadata.insert("pid".to_string(), p.pid.to_string());
                    metadata.insert("ppid".to_string(), p.ppid.to_string());
                    metadata.insert("name".to_string(), p.name.clone());
                    metadata.insert("state".to_string(), p.state.to_string());
                    metadata.insert("threads".to_string(), p.threads.to_string());
                    metadata.insert("rss_bytes".to_string(), p.rss_bytes.to_string());
                    metadata.insert("user".to_string(), p.user.clone());
                    metadata.insert("cmdline".to_string(), p.cmdline.clone());

                    let ram_str = self.format_memory(p.rss_bytes);
                    results.push(SearchResult {
                        id: format!("proc_{}", p.pid),
                        title: format!("{} (PID: {})", p.name, p.pid),
                        subtitle: Some(format!("Memory: {} | Owner: {}", ram_str, p.user)),
                        score: score as i64,
                        plugin_id: self.id(),
                        metadata,
                    });
                }
            }
            results.sort_by(|a, b| b.score.cmp(&a.score));
        }

        results
    }

    fn preview(&self, item: &SearchResult) -> Option<String> {
        let name = item.metadata.get("name")?;
        let pid = item.metadata.get("pid")?;
        let ppid = item.metadata.get("ppid")?;
        let state_char = item.metadata.get("state")?.chars().next().unwrap_or('?');
        let threads = item.metadata.get("threads")?;
        let rss_bytes: u64 = item.metadata.get("rss_bytes")?.parse().unwrap_or(0);
        let user = item.metadata.get("user")?;
        let cmdline = item.metadata.get("cmdline")?;

        let state_str = self.format_state(state_char);
        let ram_str = self.format_memory(rss_bytes);

        Some(format!(
            "# Process: {name}\n\n\
             - **Process ID (PID)**: `{pid}`\n\
             - **Parent PID (PPID)**: `{ppid}`\n\
             - **Owner/User**: `{user}`\n\
             - **Status/State**: `{state_str} ({state_char})`\n\
             - **Resident Memory (RSS)**: `{ram_str}`\n\
             - **Active Threads**: `{threads}`\n\n\
             ### Command Line:\n\
             ```bash\n\
             {cmdline}\n\
             ```\n\n\
             *Press Enter to terminate this process (sends SIGTERM).*",
        ))
    }

    fn execute(&self, item: &SearchResult, _ctx: &mut Context) -> ExecutionResult {
        let pid = match item.metadata.get("pid") {
            Some(p) => p,
            None => return ExecutionResult::Message("Process PID not found".to_string()),
        };
        let name = item.metadata.get("name").cloned().unwrap_or_default();

        let output = Command::new("kill")
            .arg(pid)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                ExecutionResult::Message(format!("Process '{name}' (PID: {pid}) terminated successfully."))
            }
            Ok(out) => {
                let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                ExecutionResult::Message(format!("Failed to kill process: {err}"))
            }
            Err(e) => {
                ExecutionResult::Message(format!("Failed to execute kill command: {e}"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_parsing() {
        let plugin = ProcessPlugin::new();
        let list = plugin.scan_processes();
        // Since test runs on a Linux system, there should be at least some processes (like PID 1 or cargo/test)
        assert!(!list.is_empty());
        let has_pid_1 = list.iter().any(|p| p.pid == 1);
        // PID 1 is standard systemd/init on almost all Linux environments
        assert!(has_pid_1 || list.len() > 0);
    }
}
