use crate::core::plugin::{Context, ExecutionResult, Plugin, SearchResult};
use crate::search::Matcher;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct InterfaceInfo {
    pub name: String,
    pub ipv4: Option<String>,
    pub ipv6: Option<String>,
    pub mac: Option<String>,
    pub state: Option<String>,
}

pub struct NetworkPlugin {
    public_ip: Arc<RwLock<Option<String>>>,
}

impl NetworkPlugin {
    pub fn new() -> Self {
        let public_ip = Arc::new(RwLock::new(None));
        let public_ip_clone = public_ip.clone();

        // Spawn background task to fetch public IP
        tokio::spawn(async move {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(3))
                .build();

            if let Ok(client) = client {
                let endpoints = vec![
                    "https://api.ipify.org",
                    "https://ifconfig.me/ip",
                    "https://icanhazip.com",
                ];
                for url in endpoints {
                    if let Ok(res) = client.get(url).send().await {
                        if res.status().is_success() {
                            if let Ok(text) = res.text().await {
                                let ip = text.trim().to_string();
                                if !ip.is_empty() {
                                    if let Ok(mut guard) = public_ip_clone.write() {
                                        *guard = Some(ip);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        Self { public_ip }
    }

    fn get_interfaces(&self) -> Vec<InterfaceInfo> {
        let mut ifaces: HashMap<String, InterfaceInfo> = HashMap::new();

        // Run ip -o addr show
        let output = Command::new("ip")
            .args(["-o", "addr", "show"])
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                for line in stdout.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 4 {
                        let name = parts[1].to_string();
                        let family = parts[2];
                        let ip_with_cidr = parts[3];
                        let ip = ip_with_cidr.split('/').next().unwrap_or(ip_with_cidr).to_string();

                        let entry = ifaces.entry(name.clone()).or_insert_with(|| {
                            // Read MAC address
                            let mac_path = PathBuf::from(format!("/sys/class/net/{}/address", name));
                            let mac = fs::read_to_string(&mac_path)
                                .map(|s| s.trim().to_string())
                                .ok();

                            // Read operstate status
                            let state_path = PathBuf::from(format!("/sys/class/net/{}/operstate", name));
                            let state = fs::read_to_string(&state_path)
                                .map(|s| s.trim().to_uppercase())
                                .ok();

                            InterfaceInfo {
                                name: name.clone(),
                                ipv4: None,
                                ipv6: None,
                                mac,
                                state,
                            }
                        });

                        if family == "inet" {
                            entry.ipv4 = Some(ip);
                        } else if family == "inet6" {
                            entry.ipv6 = Some(ip);
                        }
                    }
                }
            }
        }

        // If 'ip' command failed, fallback to /sys/class/net scanning
        if ifaces.is_empty() {
            if let Ok(entries) = fs::read_dir("/sys/class/net") {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let mac_path = entry.path().join("address");
                    let mac = fs::read_to_string(&mac_path).map(|s| s.trim().to_string()).ok();
                    let state_path = entry.path().join("operstate");
                    let state = fs::read_to_string(&state_path).map(|s| s.trim().to_uppercase()).ok();

                    ifaces.insert(name.clone(), InterfaceInfo {
                        name,
                        ipv4: None,
                        ipv6: None,
                        mac,
                        state,
                    });
                }
            }
        }

        let mut list: Vec<InterfaceInfo> = ifaces.into_values().collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }

    fn get_default_gateway(&self) -> Option<(String, String)> {
        // Runs ip route
        let output = Command::new("ip").arg("route").output().ok()?;
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 5 && parts[0] == "default" && parts[1] == "via" {
                    return Some((parts[2].to_string(), parts[4].to_string())); // (gateway_ip, dev_interface)
                }
            }
        }
        None
    }

    fn get_dns_servers(&self) -> Vec<String> {
        let mut dns = Vec::new();
        if let Ok(content) = fs::read_to_string("/etc/resolv.conf") {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("nameserver ") {
                    let ip = trimmed[11..].trim().to_string();
                    if !ip.is_empty() {
                        dns.push(ip);
                    }
                }
            }
        }
        dns
    }
}

impl Plugin for NetworkPlugin {
    fn id(&self) -> &'static str {
        "network"
    }

    fn name(&self) -> &'static str {
        "Network Info"
    }

    fn description(&self) -> &'static str {
        "View network interfaces and IP configurations"
    }

    fn search(&self, query: &str, _cache_dir: &Path) -> Vec<SearchResult> {
        let matcher = Matcher::new();
        let mut results = Vec::new();

        // 1. Interfaces
        let interfaces = self.get_interfaces();
        for iface in interfaces {
            let ipv4_str = iface.ipv4.as_deref().unwrap_or("No IPv4");
            let ipv6_str = iface.ipv6.as_deref().unwrap_or("No IPv6");
            let title = format!("Interface: {}", iface.name);
            let subtitle = format!("IPv4: {} | MAC: {}", ipv4_str, iface.mac.as_deref().unwrap_or("None"));

            let score = if query.is_empty() {
                Some(0)
            } else {
                matcher.fuzzy_match(&iface.name, query)
                    .or_else(|| iface.ipv4.as_ref().and_then(|ip| matcher.fuzzy_match(ip, query)))
            };

            if let Some(score) = score {
                let mut metadata = HashMap::new();
                metadata.insert("type".to_string(), "interface".to_string());
                metadata.insert("name".to_string(), iface.name.clone());
                metadata.insert("ipv4".to_string(), ipv4_str.to_string());
                metadata.insert("ipv6".to_string(), ipv6_str.to_string());
                metadata.insert("mac".to_string(), iface.mac.clone().unwrap_or_default());
                metadata.insert("state".to_string(), iface.state.clone().unwrap_or_default());
                metadata.insert("copy_target".to_string(), ipv4_str.to_string()); // Default copy Target

                results.push(SearchResult {
                    id: format!("net_iface_{}", iface.name),
                    title,
                    subtitle: Some(subtitle),
                    score: score as i64,
                    plugin_id: self.id(),
                    metadata,
                });
            }
        }

        // 2. Gateway
        if let Some((gateway, dev)) = self.get_default_gateway() {
            let title = "Default Gateway".to_string();
            let subtitle = format!("IP: {} via dev {}", gateway, dev);
            let score = if query.is_empty() {
                Some(0)
            } else {
                matcher.fuzzy_match("gateway", query).or_else(|| matcher.fuzzy_match(&gateway, query))
            };

            if let Some(score) = score {
                let mut metadata = HashMap::new();
                metadata.insert("type".to_string(), "gateway".to_string());
                metadata.insert("gateway".to_string(), gateway.clone());
                metadata.insert("dev".to_string(), dev.clone());
                metadata.insert("copy_target".to_string(), gateway.clone());

                results.push(SearchResult {
                    id: "net_gateway".to_string(),
                    title,
                    subtitle: Some(subtitle),
                    score: score as i64,
                    plugin_id: self.id(),
                    metadata,
                });
            }
        }

        // 3. DNS
        let dns_servers = self.get_dns_servers();
        if !dns_servers.is_empty() {
            let title = "DNS Servers".to_string();
            let dns_list = dns_servers.join(", ");
            let subtitle = format!("Servers: {}", dns_list);
            let score = if query.is_empty() {
                Some(0)
            } else {
                matcher.fuzzy_match("dns", query).or_else(|| matcher.fuzzy_match(&dns_list, query))
            };

            if let Some(score) = score {
                let mut metadata = HashMap::new();
                metadata.insert("type".to_string(), "dns".to_string());
                metadata.insert("dns".to_string(), dns_list.clone());
                metadata.insert("copy_target".to_string(), dns_servers[0].clone());

                results.push(SearchResult {
                    id: "net_dns".to_string(),
                    title,
                    subtitle: Some(subtitle),
                    score: score as i64,
                    plugin_id: self.id(),
                    metadata,
                });
            }
        }

        // 4. Public IP
        let public_ip_status = self.public_ip.read().ok()
            .and_then(|g| g.clone())
            .unwrap_or_else(|| "Fetching...".to_string());

        let title = "Public IP Address".to_string();
        let subtitle = format!("IP: {}", public_ip_status);
        let score = if query.is_empty() {
            Some(0)
        } else {
            matcher.fuzzy_match("public ip", query).or_else(|| matcher.fuzzy_match(&public_ip_status, query))
        };

        if let Some(score) = score {
            let mut metadata = HashMap::new();
            metadata.insert("type".to_string(), "public_ip".to_string());
            metadata.insert("public_ip".to_string(), public_ip_status.clone());
            metadata.insert("copy_target".to_string(), public_ip_status.clone());

            results.push(SearchResult {
                id: "net_public_ip".to_string(),
                title,
                subtitle: Some(subtitle),
                score: score as i64,
                plugin_id: self.id(),
                metadata,
            });
        }

        // Sort by match score descending
        results.sort_by(|a, b| b.score.cmp(&a.score));
        results
    }

    fn preview(&self, item: &SearchResult) -> Option<String> {
        let rtype = item.metadata.get("type")?;
        match rtype.as_str() {
            "interface" => {
                let name = item.metadata.get("name")?;
                let ipv4 = item.metadata.get("ipv4")?;
                let ipv6 = item.metadata.get("ipv6")?;
                let mac = item.metadata.get("mac")?;
                let state = item.metadata.get("state")?;

                Some(format!(
                    "# Interface: {name}\n\n\
                     - **Status**: `{state}`\n\
                     - **MAC Address**: `{mac}`\n\
                     - **IPv4 Address**: `{ipv4}`\n\
                     - **IPv6 Address**: `{ipv6}`\n\n\
                     *Press Enter to copy the IPv4 address to the clipboard.*"
                ))
            }
            "gateway" => {
                let gateway = item.metadata.get("gateway")?;
                let dev = item.metadata.get("dev")?;

                Some(format!(
                    "# Default Gateway\n\n\
                     - **Gateway IP**: `{gateway}`\n\
                     - **Routing Interface**: `{dev}`\n\n\
                     *Press Enter to copy the Gateway IP to the clipboard.*"
                ))
            }
            "dns" => {
                let dns = item.metadata.get("dns")?;

                Some(format!(
                    "# DNS Configuration\n\n\
                     - **Active Nameservers**: `{dns}`\n\n\
                     *Press Enter to copy the primary DNS server to the clipboard.*"
                ))
            }
            "public_ip" => {
                let public_ip = item.metadata.get("public_ip")?;

                Some(format!(
                    "# Public IP Address\n\n\
                     - **IP**: `{public_ip}`\n\n\
                     *Press Enter to copy the Public IP to the clipboard.*"
                ))
            }
            _ => None,
        }
    }

    fn execute(&self, item: &SearchResult, _ctx: &mut Context) -> ExecutionResult {
        if let Some(target) = item.metadata.get("copy_target") {
            if target == "No IPv4" || target == "Fetching..." || target.is_empty() {
                return ExecutionResult::Message("No address available to copy".to_string());
            }

            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                if clipboard.set_text(target.clone()).is_ok() {
                    return ExecutionResult::Message(format!("Copied address '{}' to clipboard!", target));
                }
            }
            ExecutionResult::Message("Failed to access clipboard".to_string())
        } else {
            ExecutionResult::Message("Copy target not found".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_info() {
        let plugin = NetworkPlugin::new();
        let ifaces = plugin.get_interfaces();
        // There should be at least loopback 'lo' interface on a Linux machine
        assert!(!ifaces.is_empty());
        let has_lo = ifaces.iter().any(|i| i.name == "lo");
        assert!(has_lo);
    }
}
