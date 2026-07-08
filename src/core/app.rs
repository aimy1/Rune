use crate::config::Config;
use crate::core::plugin::{Context, ExecutionResult, Plugin, SearchResult};
use crate::plugins::load_all_plugins;
use crate::storage::Storage;
use crate::ui::{draw_app, ThemeStyles, load_theme};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
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
    status_msg: Option<(String, Instant)>,
    cache_dir: PathBuf,
    storage: Storage,
    exit_requested: bool,
    command_to_run: Option<(String, Vec<String>, bool)>, // (cmd, args, in_terminal)
}

impl App {
    pub fn new(config: Config, cache_dir: PathBuf) -> Self {
        let theme_styles = ThemeStyles::from_theme(&load_theme(&config.theme.active));
        let plugins = load_all_plugins(&config);
        let storage = Storage::load(&cache_dir);

        let mut app = Self {
            config,
            theme_styles,
            plugins,
            active_plugin_idx: 0,
            query: String::new(),
            results: Vec::new(),
            selected_idx: 0,
            status_msg: None,
            cache_dir,
            storage,
            exit_requested: false,
            command_to_run: None,
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
    }

    fn handle_key_event(&mut self, key: event::KeyEvent) {
        // Clear message status if too old
        if let Some((_, time)) = self.status_msg {
            if time.elapsed() > Duration::from_secs(3) {
                self.status_msg = None;
            }
        }

        match key.code {
            KeyCode::Esc => {
                self.exit_requested = true;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.exit_requested = true;
            }
            // Tab to switch active plugin tabs
            KeyCode::Tab => {
                let count = self.plugins.len() + 1; // +1 for "All" tab
                self.active_plugin_idx = (self.active_plugin_idx + 1) % count;
                self.update_search();
            }
            KeyCode::BackTab => {
                let count = self.plugins.len() + 1;
                if self.active_plugin_idx == 0 {
                    self.active_plugin_idx = count - 1;
                } else {
                    self.active_plugin_idx -= 1;
                }
                self.update_search();
            }
            // Navigate results list
            KeyCode::Up | KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if !self.results.is_empty() {
                    if self.selected_idx == 0 {
                        self.selected_idx = self.results.len() - 1;
                    } else {
                        self.selected_idx -= 1;
                    }
                }
            }
            KeyCode::Down | KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if !self.results.is_empty() {
                    self.selected_idx = (self.selected_idx + 1) % self.results.len();
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
                    if let Some(plugin) = self.plugins.iter().find(|p| p.id() == selected_item.plugin_id) {
                        let mut ctx = Context::new();
                        let exec_res = plugin.execute(&selected_item, &mut ctx);

                        match exec_res {
                            ExecutionResult::Exit => {
                                self.exit_requested = true;
                                if let Some((cmd, args, in_term)) = ctx.command_to_run {
                                    self.command_to_run = Some((cmd, args, in_term));
                                }
                            }
                            ExecutionResult::HideAndRun(cmd, args, in_term) => {
                                self.exit_requested = true;
                                self.command_to_run = Some((cmd, args, in_term));
                            }
                            ExecutionResult::Success => {
                                // Keep open
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
            let preview_text = if !self.results.is_empty() && self.selected_idx < self.results.len() {
                let res = &self.results[self.selected_idx];
                self.plugins
                    .iter()
                    .find(|p| p.id() == res.plugin_id)
                    .and_then(|p| p.preview(res))
            } else {
                None
            };

            let status_msg_str = self.status_msg.as_ref().map(|(m, _)| m.as_str());

            terminal.draw(|f| {
                draw_app(
                    f,
                    &self.query,
                    &self.active_plugin_name(),
                    &self.results,
                    self.selected_idx,
                    preview_text,
                    &self.theme_styles,
                    status_msg_str,
                    self.plugins.len(),
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

                let _ = Command::new("sh")
                    .arg("-c")
                    .arg(format!("{} &", full_cmd))
                    .spawn();
            }
        }

        Ok(())
    }
}
