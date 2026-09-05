use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub general: GeneralConfig,
    pub theme: ThemeConfig,
    pub plugins: PluginsConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeneralConfig {
    pub shell: String,
    pub editor: String,
    pub language: String,
    pub font: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeConfig {
    pub active: String, // E.g., "catppuccin", "tokyo_night", "nord", "gruvbox", "everforest", "transparent", or file path
    #[serde(default)]
    pub transparent: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PluginsConfig {
    pub applications: bool,
    pub files: bool,
    pub file_manager: bool,
    pub commands: bool,
    pub calculator: bool,
    pub unit_converter: bool,
    pub ssh: bool,
    pub clipboard: bool,
    pub git: bool,
    pub docker: bool,
    pub systemd: bool,
    pub process: bool,
    pub network: bool,
    pub files_paths: Vec<String>,
    pub files_ignore: Vec<String>,
    pub files_max_depth: usize,
    pub file_manager_show_hidden: bool,
    pub file_manager_start_dir: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                shell: "bash".to_string(),
                editor: "nano".to_string(),
                language: "zh".to_string(),
                font: "Monospace".to_string(),
            },
            theme: ThemeConfig {
                active: "catppuccin".to_string(),
                transparent: false,
            },
            plugins: PluginsConfig {
                applications: true,
                files: true,
                file_manager: true,
                commands: true,
                calculator: true,
                unit_converter: true,
                ssh: true,
                clipboard: true,
                git: true,
                docker: false,
                systemd: false,
                process: true,
                network: true,
                files_paths: vec!["~".to_string()],
                files_ignore: vec![
                    ".git".to_string(),
                    "node_modules".to_string(),
                    "target".to_string(),
                    ".cache".to_string(),
                    ".cargo".to_string(),
                ],
                files_max_depth: 4,
                file_manager_show_hidden: false,
                file_manager_start_dir: "~".to_string(),
            },
        }
    }
}

pub fn get_config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("/home/fd/.config"))
        .join("rune")
}

pub fn get_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/home/fd/.cache"))
        .join("rune")
}

impl Config {
    pub fn save(&self) -> Result<(), std::io::Error> {
        let config_dir = get_config_dir();
        let config_path = config_dir.join("config.toml");
        let toml_str = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        fs::write(config_path, toml_str)?;
        Ok(())
    }

    pub fn load() -> Self {
        let config_dir = get_config_dir();
        let config_path = config_dir.join("config.toml");

        // Ensure directories exist
        let _ = fs::create_dir_all(&config_dir);
        let _ = fs::create_dir_all(get_cache_dir());

        // Ensure plugins directory exists
        let plugins_dir = config_dir.join("plugins");
        let _ = fs::create_dir_all(&plugins_dir);

        // Seed an example script plugin if missing
        let example_plugin_path = plugins_dir.join("hello-time.sh");
        if !example_plugin_path.exists() {
            let example_content = r##"#!/bin/bash
# Rune External Plugin Example
# This script demonstrates the Rune custom plugin JSON schema.

QUERY="$1"
NOW=$(date +"%Y-%m-%d %H:%M:%S")

cat <<EOF
[
  {
    "id": "hello_time",
    "title": "Current Date & Time",
    "subtitle": "$NOW (External script)",
    "score": 10,
    "preview": "# Hello Time Plugin\n\n- **Date**: \`$(date +%Y-%m-%d)\`\n- **Time**: \`$(date +%H:%M:%S)\`\n- **Active Search Query**: \`$QUERY\`\n\n*Press Enter to trigger a desktop notification.*",
    "execute_cmd": "notify-send",
    "execute_args": ["Rune Hello", "Current time is $NOW"],
    "run_in_terminal": false
  }
]
EOF
"##;
            let _ = fs::write(&example_plugin_path, example_content);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = fs::metadata(&example_plugin_path) {
                    let mut perms = meta.permissions();
                    perms.set_mode(0o755);
                    let _ = fs::set_permissions(&example_plugin_path, perms);
                }
            }
        }

        if !config_path.exists() {
            let default_config = Config::default();
            if let Ok(toml_str) = toml::to_string_pretty(&default_config) {
                let _ = fs::write(&config_path, toml_str);
            }
            default_config
        } else {
            match fs::read_to_string(&config_path) {
                Ok(content) => toml::from_str(&content).unwrap_or_else(|err| {
                    eprintln!("Warning: Failed to parse config.toml: {err}. Using defaults.");
                    Config::default()
                }),
                Err(_) => Config::default(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_config_default_transparent() {
        let config = Config::default();
        assert!(!config.theme.transparent);
    }

    #[test]
    fn test_theme_config_deserialize_with_and_without_transparent() {
        let toml_without_transparent = r#"
            [general]
            shell = "bash"
            editor = "nano"
            language = "zh"
            font = "Monospace"

            [theme]
            active = "tokyo_night"

            [plugins]
            applications = true
            files = true
            file_manager = true
            commands = true
            calculator = true
            unit_converter = true
            ssh = true
            clipboard = true
            git = true
            docker = false
            systemd = false
            process = true
            network = true
            files_paths = ["~"]
            files_ignore = []
            files_max_depth = 4
            file_manager_show_hidden = false
            file_manager_start_dir = "~"
        "#;
        let parsed: Config = toml::from_str(toml_without_transparent).expect("parse without transparent");
        assert_eq!(parsed.theme.active, "tokyo_night");
        assert!(!parsed.theme.transparent);

        let toml_with_transparent = r#"
            [general]
            shell = "bash"
            editor = "nano"
            language = "zh"
            font = "Monospace"

            [theme]
            active = "catppuccin"
            transparent = true

            [plugins]
            applications = true
            files = true
            file_manager = true
            commands = true
            calculator = true
            unit_converter = true
            ssh = true
            clipboard = true
            git = true
            docker = false
            systemd = false
            process = true
            network = true
            files_paths = ["~"]
            files_ignore = []
            files_max_depth = 4
            file_manager_show_hidden = false
            file_manager_start_dir = "~"
        "#;
        let parsed_trans: Config = toml::from_str(toml_with_transparent).expect("parse with transparent");
        assert_eq!(parsed_trans.theme.active, "catppuccin");
        assert!(parsed_trans.theme.transparent);
    }
}
