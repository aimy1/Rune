pub mod ai;
pub mod applications;
pub mod calculator;
pub mod commands;
pub mod docker;
pub mod external;
pub mod files;
pub mod file_manager;
pub mod git;
pub mod ssh;
pub mod clipboard;
pub mod systemd;
pub mod unit_converter;
pub mod process;
pub mod network;

pub use ai::AiPlugin;
pub use applications::ApplicationsPlugin;
pub use calculator::CalculatorPlugin;
pub use commands::CommandsPlugin;
pub use docker::DockerPlugin;
pub use external::ExternalPlugin;
pub use files::FilesPlugin;
pub use file_manager::FileManagerPlugin;
pub use git::GitPlugin;
pub use ssh::SshPlugin;
pub use clipboard::{ClipboardPlugin, run_clipboard_daemon};
pub use systemd::SystemdPlugin;
pub use unit_converter::UnitConverterPlugin;
pub use process::ProcessPlugin;
pub use network::NetworkPlugin;

use crate::config::{Config, get_cache_dir, get_config_dir};
use crate::core::plugin::Plugin;

pub fn load_all_plugins(config: &Config) -> Vec<Box<dyn Plugin>> {
    let mut plugins: Vec<Box<dyn Plugin>> = Vec::new();
    let cache_dir = get_cache_dir();

    // 1. Load built-in plugins if enabled in config
    if config.plugins.applications {
        plugins.push(Box::new(ApplicationsPlugin::new(&cache_dir)));
    }
    if config.plugins.files {
        plugins.push(Box::new(FilesPlugin::new(
            &cache_dir,
            config.plugins.files_paths.clone(),
            config.plugins.files_ignore.clone(),
            config.plugins.files_max_depth,
        )));
    }
    if config.plugins.commands {
        plugins.push(Box::new(CommandsPlugin::new()));
    }
    if config.plugins.calculator {
        plugins.push(Box::new(CalculatorPlugin::new()));
    }
    if config.plugins.unit_converter {
        plugins.push(Box::new(UnitConverterPlugin::new()));
    }
    if config.plugins.ssh {
        plugins.push(Box::new(SshPlugin::new()));
    }
    if config.plugins.clipboard {
        plugins.push(Box::new(ClipboardPlugin::new(&cache_dir)));
    }
    if config.plugins.git {
        plugins.push(Box::new(GitPlugin::new()));
    }
    if config.plugins.docker {
        plugins.push(Box::new(DockerPlugin::new()));
    }
    if config.plugins.systemd {
        plugins.push(Box::new(SystemdPlugin::new()));
    }
    if config.plugins.ai {
        plugins.push(Box::new(AiPlugin::new(
            config.plugins.ai_provider.clone(),
            config.plugins.ai_api_key.clone(),
            config.plugins.ai_model.clone(),
            config.plugins.ai_api_url.clone(),
        )));
    }
    if config.plugins.process {
        plugins.push(Box::new(ProcessPlugin::new()));
    }
    if config.plugins.network {
        plugins.push(Box::new(NetworkPlugin::new()));
    }

    // 2. Load external plugins from ~/.config/rune/plugins/
    let config_dir = get_config_dir();
    let plugins_dir = config_dir.join("plugins");
    let external_plugins = ExternalPlugin::scan_external_plugins(&plugins_dir);
    for p in external_plugins {
        plugins.push(Box::new(p));
    }

    plugins
}
