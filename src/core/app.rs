use crate::config::Config;
use crate::core::plugin::{Context, ExecutionResult, Plugin, SearchResult};
use crate::plugins::load_all_plugins;
use crate::storage::Storage;
use crate::ui::{draw_app, ThemeStyles, load_theme};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::widgets::ListState;
use ratatui::Terminal;
use std::io;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

pub struct App {
    config: Config,
    theme_styles: ThemeStyles,
    plugins: Vec<Box<dyn Plugin>>,
    active_plugin_idx: usize, // 0 = All plugins combined, 1..N = individual plugins
    query: String,
    results: Vec<SearchResult>,
    selected_idx: usize,
    list_state: ListState,
    preview_scroll: u16,
    status_msg: Option<(String, Instant)>,
    cache_dir: PathBuf,
    storage: Storage,
    exit_requested: bool,
    command_to_run: Option<(String, Vec<String>, bool)>, // (cmd, args, in_terminal)
    // Settings configuration
    settings_open: bool,
    settings_focused_pane: usize, // 0 = Categories pane, 1 = Options pane
    settings_selected_category: usize,
    settings_selected_option: usize,
    scanned_fonts: Vec<String>,
    settings_input_mode: bool,
    settings_input_buffer: String,
    file_manager: crate::plugins::FileManagerPlugin,
    file_manager_open: bool,
    main_focus_pane: usize,
}

fn scan_monospace_fonts() -> Vec<String> {
    let output = Command::new("fc-list")
        .args([":spacing=100", "family"])
        .output();
    
    let mut fonts = Vec::new();
    if let Ok(out) = output {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let parts: Vec<&str> = line.split(',').collect();
            if let Some(first) = parts.first() {
                let family = first.split(':').next().unwrap_or(first).trim().to_string();
                if !family.is_empty() && !fonts.contains(&family) {
                    fonts.push(family);
                }
            }
        }
    }
    
    if fonts.is_empty() {
        fonts = vec![
            "FiraCode Nerd Font".to_string(),
            "JetBrainsMono Nerd Font".to_string(),
            "Source Code Pro".to_string(),
            "Monospace".to_string(),
        ];
    }
    
    fonts.sort();
    fonts
}

impl App {
    pub fn new(config: Config, cache_dir: PathBuf) -> Self {
        let theme_styles = ThemeStyles::from_theme(&load_theme(&config.theme.active));
        let plugins = load_all_plugins(&config);
        let storage = Storage::load(&cache_dir);

        let mut list_state = ListState::default();
        list_state.select(Some(0));

        let scanned_fonts = scan_monospace_fonts();

        let show_hidden = config.plugins.file_manager_show_hidden;
        let start_dir = config.plugins.file_manager_start_dir.clone();

        let mut app = Self {
            config,
            theme_styles,
            plugins,
            active_plugin_idx: 0,
            query: String::new(),
            results: Vec::new(),
            selected_idx: 0,
            list_state,
            preview_scroll: 0,
            status_msg: None,
            cache_dir,
            storage,
            exit_requested: false,
            command_to_run: None,
            settings_open: false,
            settings_focused_pane: 0,
            settings_selected_category: 0,
            settings_selected_option: 0,
            scanned_fonts,
            settings_input_mode: false,
            settings_input_buffer: String::new(),
            file_manager: crate::plugins::FileManagerPlugin::new(
                show_hidden,
                &start_dir,
            ),
            file_manager_open: false,
            main_focus_pane: 0,
        };

        app.update_search();
        app
    }

    fn active_plugin_name(&self) -> String {
        if self.active_plugin_idx == 0 {
            "All Plugins".to_string()
        } else {
            self.plugins[self.active_plugin_idx - 1].name().to_string()
        }
    }

    fn update_search(&mut self) {
        if self.file_manager_open {
            self.results = self.file_manager.search(&self.query, &self.cache_dir);
            self.selected_idx = 0;
            self.list_state.select(Some(0));
            self.preview_scroll = 0;
            return;
        }
        let mut combined = Vec::new();
        let query = &self.query;

        if self.active_plugin_idx == 0 {
            // Search all plugins
            for plugin in &self.plugins {
                let mut res = plugin.search(query, &self.cache_dir);
                // Apply frecency boosting
                for item in &mut res {
                    let key = format!("{}:{}", item.plugin_id, item.id);
                    item.score += self.storage.get_frecency_bonus(&key);
                }
                combined.extend(res);
            }
            // Sort by match score descending
            combined.sort_by(|a, b| b.score.cmp(&a.score));
        } else {
            // Search single active plugin
            let plugin = &self.plugins[self.active_plugin_idx - 1];
            let mut res = plugin.search(query, &self.cache_dir);
            for item in &mut res {
                let key = format!("{}:{}", item.plugin_id, item.id);
                item.score += self.storage.get_frecency_bonus(&key);
            }
            res.sort_by(|a, b| b.score.cmp(&a.score));
            combined = res;
        }

        self.results = combined;
        self.selected_idx = 0; // Reset selection index
        self.list_state.select(Some(0));
        self.preview_scroll = 0;
    }

    fn get_options_count(&self) -> usize {
        match self.settings_selected_category {
            0 => 5, // TUI themes
            1 => 2, // UI language
            2 => 5, // editor (+ custom)
            3 => 4, // Terminal Shell (bash, zsh, sh, custom)
            4 => self.scanned_fonts.len(), // Monospace fonts
            5 => 14, // plugins toggles (updated from 13 to 14)
            6 => 3, // File Search Settings (Max Depth, Search Paths, Ignore Paths)
            7 => 2, // File Manager Settings (Show Hidden, Start Dir)
            8 => 3, // AI providers
            9 => 3, // AI configs (Key, Model, URL)
            10 => 0, // About (0 options, focus stays on category pane)
            _ => 0,
        }
    }

    fn apply_and_save_setting(&mut self) {
        match self.settings_selected_category {
            0 => {
                let themes = vec![
                    "catppuccin".to_string(),
                    "tokyo_night".to_string(),
                    "nord".to_string(),
                    "gruvbox".to_string(),
                    "everforest".to_string(),
                ];
                if self.settings_selected_option < themes.len() {
                    let chosen = themes[self.settings_selected_option].clone();
                    self.config.theme.active = chosen;
                    // Live theme update!
                    self.theme_styles = ThemeStyles::from_theme(&load_theme(&self.config.theme.active));
                }
            }
            1 => {
                let langs = vec!["zh".to_string(), "en".to_string()];
                if self.settings_selected_option < langs.len() {
                    let chosen = langs[self.settings_selected_option].clone();
                    self.config.general.language = chosen;
                }
            }
            2 => {
                let editors = vec!["nano".to_string(), "vim".to_string(), "nvim".to_string(), "hx".to_string()];
                if self.settings_selected_option < editors.len() {
                    let chosen = editors[self.settings_selected_option].clone();
                    self.config.general.editor = chosen;
                }
            }
            3 => {
                let shells = vec!["bash".to_string(), "zsh".to_string(), "sh".to_string()];
                if self.settings_selected_option < shells.len() {
                    let chosen = shells[self.settings_selected_option].clone();
                    self.config.general.shell = chosen;
                }
            }
            4 => {
                if self.settings_selected_option < self.scanned_fonts.len() {
                    let chosen = self.scanned_fonts[self.settings_selected_option].clone();
                    self.config.general.font = chosen;
                }
            }
            5 => {
                // Toggle plugin enable state on/off
                match self.settings_selected_option {
                    0 => self.config.plugins.applications = !self.config.plugins.applications,
                    1 => self.config.plugins.files = !self.config.plugins.files,
                    2 => self.config.plugins.file_manager = !self.config.plugins.file_manager,
                    3 => self.config.plugins.commands = !self.config.plugins.commands,
                    4 => self.config.plugins.calculator = !self.config.plugins.calculator,
                    5 => self.config.plugins.unit_converter = !self.config.plugins.unit_converter,
                    6 => self.config.plugins.ssh = !self.config.plugins.ssh,
                    7 => self.config.plugins.clipboard = !self.config.plugins.clipboard,
                    8 => self.config.plugins.git = !self.config.plugins.git,
                    9 => self.config.plugins.docker = !self.config.plugins.docker,
                    10 => self.config.plugins.systemd = !self.config.plugins.systemd,
                    11 => self.config.plugins.ai = !self.config.plugins.ai,
                    12 => self.config.plugins.process = !self.config.plugins.process,
                    13 => self.config.plugins.network = !self.config.plugins.network,
                    _ => {}
                }
                // Live reload active plugins list
                self.plugins = load_all_plugins(&self.config);
                self.active_plugin_idx = 0; // reset to All tab
                self.update_search();
            }
            7 => {
                // File Manager Option 0: Show Hidden toggle
                if self.settings_selected_option == 0 {
                    self.config.plugins.file_manager_show_hidden = !self.config.plugins.file_manager_show_hidden;
                    self.file_manager.set_show_hidden(self.config.plugins.file_manager_show_hidden);
                }
            }
            8 => {
                let providers = vec!["openai".to_string(), "gemini".to_string(), "ollama".to_string()];
                if self.settings_selected_option < providers.len() {
                    let chosen = providers[self.settings_selected_option].clone();
                    self.config.plugins.ai_provider = chosen.clone();
                    
                    // Set correct default model and completions API URL for the chosen service provider
                    match chosen.as_str() {
                        "openai" => {
                            self.config.plugins.ai_model = "gpt-4o-mini".to_string();
                            self.config.plugins.ai_api_url = "https://api.openai.com/v1/chat/completions".to_string();
                        }
                        "gemini" => {
                            self.config.plugins.ai_model = "gemini-1.5-flash".to_string();
                            self.config.plugins.ai_api_url = "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions".to_string();
                        }
                        "ollama" => {
                            self.config.plugins.ai_model = "llama3".to_string();
                            self.config.plugins.ai_api_url = "http://localhost:11434/api/generate".to_string();
                        }
                        _ => {}
                    }
                    
                    // Reload AI plugin live with new configurations
                    self.plugins = load_all_plugins(&self.config);
                }
            }
            _ => {}
        }

        // Save configuration to disk
        if let Ok(_) = self.config.save() {
            let is_zh = self.config.general.language == "zh";
            let msg = if is_zh {
                "设置已成功保存并实时生效！"
            } else {
                "Settings saved and applied successfully!"
            };
            self.status_msg = Some((msg.to_string(), Instant::now()));
        }
    }

    fn save_custom_input(&mut self) {
        let input = self.settings_input_buffer.trim().to_string();
        match self.settings_selected_category {
            2 => {
                if !input.is_empty() {
                    self.config.general.editor = input;
                }
            }
            3 => {
                if !input.is_empty() {
                    self.config.general.shell = input;
                }
            }
            6 => {
                match self.settings_selected_option {
                    0 => {
                        if let Ok(depth) = input.parse::<usize>() {
                            self.config.plugins.files_max_depth = depth;
                            // Invalidate the search cache to trigger a fresh walkdir sweep with new depth setting
                            let cache_file = self.cache_dir.join("files.txt");
                            let _ = std::fs::remove_file(cache_file);
                            // Reload files plugin live
                            self.plugins = load_all_plugins(&self.config);
                        }
                    }
                    1 => {
                        let paths: Vec<String> = input.split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                        self.config.plugins.files_paths = paths;
                        // Reload files plugin live
                        self.plugins = load_all_plugins(&self.config);
                    }
                    2 => {
                        let ignore: Vec<String> = input.split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                        self.config.plugins.files_ignore = ignore;
                        // Reload files plugin live
                        self.plugins = load_all_plugins(&self.config);
                    }
                    _ => {}
                }
            }
            7 => {
                if self.settings_selected_option == 1 {
                    self.config.plugins.file_manager_start_dir = input;
                }
            }
            9 => {
                match self.settings_selected_option {
                    0 => self.config.plugins.ai_api_key = input,
                    1 => {
                        if !input.is_empty() {
                            self.config.plugins.ai_model = input;
                        }
                    }
                    2 => {
                        if !input.is_empty() {
                            self.config.plugins.ai_api_url = input;
                        }
                    }
                    _ => {}
                }
                // Reload AI plugin live
                self.plugins = load_all_plugins(&self.config);
            }
            _ => {}
        }

        if let Ok(_) = self.config.save() {
            let is_zh = self.config.general.language == "zh";
            let msg = if is_zh {
                "设置已成功保存并实时生效！"
            } else {
                "Settings saved and applied successfully!"
            };
            self.status_msg = Some((msg.to_string(), Instant::now()));
        }
    }

    fn handle_key_event(&mut self, key: event::KeyEvent) {
        // Clear message status if too old
        if let Some((_, time)) = self.status_msg {
            if time.elapsed() > Duration::from_secs(3) {
                self.status_msg = None;
            }
        }

        // Global F1 key listener for system settings
        if key.code == KeyCode::F(1) {
            self.settings_open = !self.settings_open;
            if self.settings_open {
                self.file_manager_open = false;
            }
            self.settings_focused_pane = 0;
            self.settings_selected_category = 0;
            self.settings_selected_option = 0;
            self.query.clear();
            self.update_search();
            return;
        }

        // Global F2 key listener for file manager
        if key.code == KeyCode::F(2) && self.config.plugins.file_manager {
            self.file_manager_open = !self.file_manager_open;
            if self.file_manager_open {
                self.settings_open = false;
            }
            self.query.clear();
            self.update_search();
            return;
        }

        if self.settings_open {
            if self.settings_input_mode {
                match key.code {
                    KeyCode::Esc => {
                        self.settings_input_mode = false;
                        self.settings_input_buffer.clear();
                    }
                    KeyCode::Backspace => {
                        self.settings_input_buffer.pop();
                    }
                    KeyCode::Char(c) => {
                        self.settings_input_buffer.push(c);
                    }
                    KeyCode::Enter => {
                        self.save_custom_input();
                        self.settings_input_mode = false;
                        self.settings_input_buffer.clear();
                    }
                    _ => {}
                }
                return;
            }

            match key.code {
                KeyCode::Esc => {
                    self.settings_open = false;
                }
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.settings_open = false;
                }
                KeyCode::Left => {
                    self.settings_focused_pane = 0;
                }
                KeyCode::Right => {
                    if self.settings_selected_category != 10 {
                        self.settings_focused_pane = 1;
                    }
                }
                KeyCode::Tab => {
                    if self.settings_selected_category != 10 {
                        self.settings_focused_pane = (self.settings_focused_pane + 1) % 2;
                    } else {
                        self.settings_focused_pane = 0;
                    }
                }
                KeyCode::BackTab => {
                    if self.settings_selected_category != 10 {
                        self.settings_focused_pane = (self.settings_focused_pane + 1) % 2;
                    } else {
                        self.settings_focused_pane = 0;
                    }
                }
                KeyCode::Up => {
                    if self.settings_focused_pane == 0 {
                        if self.settings_selected_category == 0 {
                            self.settings_selected_category = 10;
                        } else {
                            self.settings_selected_category -= 1;
                        }
                        self.settings_selected_option = 0;
                    } else {
                        let count = self.get_options_count();
                        if count > 0 {
                            if self.settings_selected_option == 0 {
                                self.settings_selected_option = count - 1;
                            } else {
                                self.settings_selected_option -= 1;
                            }
                        }
                    }
                }
                KeyCode::Down => {
                    if self.settings_focused_pane == 0 {
                        self.settings_selected_category = (self.settings_selected_category + 1) % 11;
                        self.settings_selected_option = 0;
                    } else {
                        let count = self.get_options_count();
                        if count > 0 {
                            self.settings_selected_option = (self.settings_selected_option + 1) % count;
                        }
                    }
                }
                KeyCode::Enter => {
                    if self.settings_focused_pane == 1 {
                        let is_custom_trigger = match self.settings_selected_category {
                            2 => self.settings_selected_option == 4, // Custom Editor
                            3 => self.settings_selected_option == 3, // Custom Shell
                            6 => true, // File Search Settings: Max Depth, Search Paths, Ignore Paths
                            7 => self.settings_selected_option == 1, // File Manager Start Dir
                            9 => true, // Any AI Config field
                            _ => false,
                        };
                        
                        if is_custom_trigger {
                            self.settings_input_mode = true;
                            self.settings_input_buffer = match self.settings_selected_category {
                                2 => self.config.general.editor.clone(),
                                3 => self.config.general.shell.clone(),
                                6 => match self.settings_selected_option {
                                    0 => self.config.plugins.files_max_depth.to_string(),
                                    1 => self.config.plugins.files_paths.join(","),
                                    2 => self.config.plugins.files_ignore.join(","),
                                    _ => String::new(),
                                },
                                7 => match self.settings_selected_option {
                                    1 => self.config.plugins.file_manager_start_dir.clone(),
                                    _ => String::new(),
                                },
                                9 => match self.settings_selected_option {
                                    0 => self.config.plugins.ai_api_key.clone(),
                                    1 => self.config.plugins.ai_model.clone(),
                                    2 => self.config.plugins.ai_api_url.clone(),
                                    _ => String::new(),
                                },
                                _ => String::new(),
                            };
                        } else {
                            self.apply_and_save_setting();
                        }
                    } else {
                        if self.settings_selected_category != 10 {
                            self.settings_focused_pane = 1;
                            self.settings_selected_option = 0;
                        }
                    }
                }
                _ => {}
            }
            return;
        }

        // Let the active plugin intercept the key if it wants to!
        if self.file_manager_open {
            let selected_item = if !self.results.is_empty() && self.selected_idx < self.results.len() {
                Some(&self.results[self.selected_idx])
            } else {
                None
            };

            let mut ctx = Context::new(self.config.general.editor.clone(), self.config.general.shell.clone());
            if self.file_manager.handle_key(key, &self.query, selected_item, &mut ctx) {
                // Keep config in sync if show_hidden is changed via hotkey Alt-h
                let current_fm_show_hidden = self.file_manager.get_show_hidden();
                if current_fm_show_hidden != self.config.plugins.file_manager_show_hidden {
                    self.config.plugins.file_manager_show_hidden = current_fm_show_hidden;
                    let _ = self.config.save();
                }

                if ctx.exit_requested {
                    if let Some((cmd, args, in_term)) = ctx.command_to_run {
                        self.command_to_run = Some((cmd, args, in_term));
                    } else {
                        self.exit_requested = true;
                    }
                }
                if let Some(msg) = ctx.message {
                    self.status_msg = Some((msg, Instant::now()));
                }
                if let Some(new_q) = ctx.new_query {
                    self.query = new_q;
                }
                if ctx.refresh_search {
                    self.update_search();
                    if let Some(ref target) = ctx.focus_target {
                        if let Some(pos) = self.results.iter().position(|r| &r.title == target) {
                            self.selected_idx = pos;
                            self.list_state.select(Some(pos));
                        }
                    }
                }
                return;
            }
        } else if self.active_plugin_idx > 0 {
            let plugin = &self.plugins[self.active_plugin_idx - 1];
            let selected_item = if !self.results.is_empty() && self.selected_idx < self.results.len() {
                Some(&self.results[self.selected_idx])
            } else {
                None
            };

            let mut ctx = Context::new(self.config.general.editor.clone(), self.config.general.shell.clone());
            if plugin.handle_key(key, &self.query, selected_item, &mut ctx) {
                if ctx.exit_requested {
                    if let Some((cmd, args, in_term)) = ctx.command_to_run {
                        self.command_to_run = Some((cmd, args, in_term));
                    } else {
                        self.exit_requested = true;
                    }
                }
                if let Some(msg) = ctx.message {
                    self.status_msg = Some((msg, Instant::now()));
                }
                if let Some(new_q) = ctx.new_query {
                    self.query = new_q;
                }
                if ctx.refresh_search {
                    self.update_search();
                    if let Some(ref target) = ctx.focus_target {
                        if let Some(pos) = self.results.iter().position(|r| &r.title == target) {
                            self.selected_idx = pos;
                            self.list_state.select(Some(pos));
                        }
                    }
                }
                return;
            }
        }

        match key.code {
            KeyCode::Esc => {
                if self.file_manager_open {
                    self.file_manager_open = false;
                    self.query.clear();
                    self.update_search();
                } else {
                    self.exit_requested = true;
                }
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.exit_requested = true;
            }
            // Tab to switch active plugin tabs (modes)
            KeyCode::Tab => {
                let count = self.plugins.len() + 1; // +1 for "All" tab
                self.active_plugin_idx = (self.active_plugin_idx + 1) % count;
                self.update_search();
            }
            KeyCode::BackTab => {
                let count = self.plugins.len() + 1; // +1 for "All" tab
                if self.active_plugin_idx == 0 {
                    self.active_plugin_idx = count - 1;
                } else {
                    self.active_plugin_idx -= 1;
                }
                self.update_search();
            }
            // Scroll preview pane using Shift-Up/Down, PageUp/PageDown, or Alt-j/k
            KeyCode::PageUp => {
                self.preview_scroll = self.preview_scroll.saturating_sub(5);
            }
            KeyCode::PageDown => {
                self.preview_scroll = self.preview_scroll.saturating_add(5);
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.preview_scroll = self.preview_scroll.saturating_sub(1);
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.preview_scroll = self.preview_scroll.saturating_add(1);
            }
            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::ALT) => {
                self.preview_scroll = self.preview_scroll.saturating_sub(1);
            }
            KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::ALT) => {
                self.preview_scroll = self.preview_scroll.saturating_add(1);
            }
            // Navigate results list using Arrow keys
            KeyCode::Up => {
                if !self.results.is_empty() {
                    if self.selected_idx == 0 {
                        self.selected_idx = self.results.len() - 1;
                    } else {
                        self.selected_idx -= 1;
                    }
                    self.list_state.select(Some(self.selected_idx));
                    self.preview_scroll = 0;
                }
            }
            KeyCode::Down => {
                if !self.results.is_empty() {
                    self.selected_idx = (self.selected_idx + 1) % self.results.len();
                    self.list_state.select(Some(self.selected_idx));
                    self.preview_scroll = 0;
                }
            }
            // Alternative navigation using Emacs (Ctrl-p/Ctrl-n) or Vim (Ctrl-k/Ctrl-j) keys
            KeyCode::Char('p') | KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if !self.results.is_empty() {
                    if self.selected_idx == 0 {
                        self.selected_idx = self.results.len() - 1;
                    } else {
                        self.selected_idx -= 1;
                    }
                    self.list_state.select(Some(self.selected_idx));
                    self.preview_scroll = 0;
                }
            }
            KeyCode::Char('n') | KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if !self.results.is_empty() {
                    self.selected_idx = (self.selected_idx + 1) % self.results.len();
                    self.list_state.select(Some(self.selected_idx));
                    self.preview_scroll = 0;
                }
            }
            // Input editing keys
            KeyCode::Backspace => {
                self.query.pop();
                self.update_search();
            }
            KeyCode::Char(c) => {
                self.query.push(c);
                self.update_search();
            }
            // Execute chosen item
            KeyCode::Enter => {
                if !self.results.is_empty() && self.selected_idx < self.results.len() {
                    let selected_item = self.results[self.selected_idx].clone();
                    
                    // Record launch key for frecency tracking
                    let key = format!("{}:{}", selected_item.plugin_id, selected_item.id);
                    self.storage.record_use(&key);
                    self.storage.save(&self.cache_dir);

                    // Find corresponding plugin
                    let plugin_opt: Option<&dyn Plugin> = if self.file_manager_open && selected_item.plugin_id == "file_manager" {
                        Some(&self.file_manager)
                    } else {
                        self.plugins.iter().find(|p| p.id() == selected_item.plugin_id).map(|p| p.as_ref())
                    };

                    if let Some(plugin) = plugin_opt {
                        let mut ctx = Context::new(self.config.general.editor.clone(), self.config.general.shell.clone());
                        let exec_res = plugin.execute(&selected_item, &mut ctx);

                        match exec_res {
                            ExecutionResult::Exit => {
                                if let Some((cmd, args, in_term)) = ctx.command_to_run {
                                    self.command_to_run = Some((cmd, args, in_term));
                                } else {
                                    self.exit_requested = true;
                                }
                            }
                            ExecutionResult::HideAndRun(cmd, args, in_term) => {
                                self.command_to_run = Some((cmd, args, in_term));
                            }
                            ExecutionResult::Success => {
                                // Keep open
                                if let Some(new_q) = ctx.new_query {
                                    self.query = new_q;
                                }
                                if ctx.refresh_search {
                                    self.update_search();
                                }
                            }
                            ExecutionResult::Message(msg) => {
                                self.status_msg = Some((msg, Instant::now()));
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        crossterm::execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let mut last_tick = Instant::now();
        let tick_rate = Duration::from_millis(100);

        while !self.exit_requested {
            // Draw UI
            let preview_text = if self.file_manager_open {
                if !self.results.is_empty() && self.selected_idx < self.results.len() {
                    let res = &self.results[self.selected_idx];
                    self.file_manager.preview(res)
                } else {
                    None
                }
            } else if !self.results.is_empty() && self.selected_idx < self.results.len() {
                let res = &self.results[self.selected_idx];
                self.plugins
                    .iter()
                    .find(|p| p.id() == res.plugin_id)
                    .and_then(|p| p.preview(res))
            } else {
                None
            };

            let status_msg_str = self.status_msg.as_ref().map(|(m, _)| m.clone());

            terminal.draw(|f| {
                draw_app(
                    f,
                    &self.query,
                    &self.active_plugin_name(),
                    &self.results,
                    &mut self.list_state,
                    preview_text,
                    self.preview_scroll,
                    &self.theme_styles,
                    status_msg_str.as_deref(),
                    self.plugins.len(),
                    self.settings_open,
                    self.settings_focused_pane,
                    self.settings_selected_category,
                    self.settings_selected_option,
                    &self.scanned_fonts,
                    &self.config,
                    self.settings_input_mode,
                    &self.settings_input_buffer,
                    self.file_manager_open,
                    self.main_focus_pane,
                );
            })?;

            // Poll events
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    self.handle_key_event(key);
                }
            }

            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
                // Clear old message
                if let Some((_, time)) = self.status_msg {
                    if time.elapsed() > Duration::from_secs(3) {
                        self.status_msg = None;
                    }
                }
            }

            if self.command_to_run.is_some() {
                if let Some((cmd, args, in_terminal)) = self.command_to_run.take() {
                    let full_cmd = if args.is_empty() {
                        cmd.clone()
                    } else {
                        format!("{} {}", cmd, args.join(" "))
                    };

                    if in_terminal {
                        // Temporarily suspend terminal raw mode and alternate screen
                        disable_raw_mode()?;
                        crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                        terminal.show_cursor()?;

                        // Run command
                        let mut child = Command::new(&self.config.general.shell)
                            .arg("-c")
                            .arg(&full_cmd)
                            .spawn()?;
                        let _ = child.wait()?;

                        // Prompt user to press Enter before returning to Rune
                        println!("\n[Rune: Process exited. Press Enter to return to Rune]");
                        let mut buf = String::new();
                        let _ = io::stdin().read_line(&mut buf);

                        // Restore terminal raw mode and alternate screen
                        enable_raw_mode()?;
                        crossterm::execute!(terminal.backend_mut(), EnterAlternateScreen)?;
                        terminal.clear()?;
                    } else {
                        // Run background detached command
                        let _ = Command::new(&self.config.general.shell)
                            .arg("-c")
                            .arg(format!("{} &", full_cmd))
                            .stdout(std::process::Stdio::null())
                            .stderr(std::process::Stdio::null())
                            .spawn();
                        let msg = format!("Launched: {}", full_cmd);
                        self.status_msg = Some((msg, Instant::now()));
                    }
                    self.update_search();
                }
            }
        }

        // Cleanup Terminal raw state
        disable_raw_mode()?;
        crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        // Post-quit action: Run deferred command if any
        if let Some((cmd, args, in_terminal)) = self.command_to_run.take() {
            if in_terminal {
                // Interactive command running in foreground
                let full_cmd = if args.is_empty() {
                    cmd
                } else {
                    format!("{} {}", cmd, args.join(" "))
                };

                let mut child = Command::new(&self.config.general.shell)
                    .arg("-c")
                    .arg(&full_cmd)
                    .spawn()?;
                let _ = child.wait()?;
                
                // Let user read final terminal output before clean exit
                println!("\n[Rune: Process exited. Press Enter to return to terminal]");
                let mut buf = String::new();
                let _ = io::stdin().read_line(&mut buf);
            } else {
                // Detached GUI/background spawn command
                let full_cmd = if args.is_empty() {
                    cmd
                } else {
                    format!("{} {}", cmd, args.join(" "))
                };

                let _ = Command::new(&self.config.general.shell)
                    .arg("-c")
                    .arg(format!("{} &", full_cmd))
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn();
            }
        }

        Ok(())
    }
}
