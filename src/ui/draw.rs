use crate::config::Config;
use crate::core::plugin::SearchResult;
use crate::ui::ThemeStyles;
use image::GenericImageView;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Frame;
use std::path::Path;

fn draw_image_in_preview(
    path_str: &str,
    theme: &ThemeStyles,
    max_width: u32,
    max_height: u32,
) -> Vec<Line<'static>> {
    let path = Path::new(path_str);
    if !path.exists() {
        return vec![Line::from(Span::styled(
            format!("[Image not found: {}]", path_str),
            Style::default().fg(theme.error),
        ))];
    }

    match image::open(path) {
        Ok(img) => {
            let (width, height) = img.dimensions();
            if width == 0 || height == 0 {
                return vec![Line::from("[Invalid image dimensions]")];
            }
            let aspect = width as f32 / height as f32;

            let mut new_width = max_width;
            let mut new_height = (new_width as f32 / aspect / 2.0) as u32;

            if new_height > max_height {
                new_height = max_height;
                new_width = (new_height as f32 * aspect * 2.0) as u32;
            }

            if new_width == 0 {
                new_width = 1;
            }
            if new_height == 0 {
                new_height = 1;
            }

            let resized = img.resize_exact(
                new_width,
                new_height * 2,
                image::imageops::FilterType::Nearest,
            );
            let mut lines = Vec::new();

            for y in (0..new_height * 2).step_by(2) {
                let mut spans = Vec::new();
                for x in 0..new_width {
                    let top_pixel = resized.get_pixel(x, y);
                    let bottom_pixel = if y + 1 < new_height * 2 {
                        resized.get_pixel(x, y + 1)
                    } else {
                        top_pixel
                    };

                    let top_alpha = top_pixel[3];
                    let bottom_alpha = bottom_pixel[3];

                    let top_color = Color::Rgb(top_pixel[0], top_pixel[1], top_pixel[2]);
                    let bottom_color = Color::Rgb(bottom_pixel[0], bottom_pixel[1], bottom_pixel[2]);

                    if top_alpha < 50 && bottom_alpha < 50 {
                        spans.push(Span::raw(" "));
                    } else if top_alpha < 50 {
                        spans.push(Span::styled("▄", Style::default().fg(bottom_color)));
                    } else if bottom_alpha < 50 {
                        spans.push(Span::styled("▀", Style::default().fg(top_color)));
                    } else {
                        spans.push(Span::styled(
                            "▄",
                            Style::default().bg(top_color).fg(bottom_color),
                        ));
                    }
                }
                lines.push(Line::from(spans));
            }
            lines
        }
        Err(e) => vec![Line::from(Span::styled(
            format!("[Failed to load image: {}]", e),
            Style::default().fg(theme.error),
        ))],
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn parse_markdown_line(line: &str, theme: &ThemeStyles) -> Line<'static> {
    let trimmed = line.trim();
    if trimmed.starts_with("# ") {
        Line::from(Span::styled(
            trimmed[2..].to_string(),
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        ))
    } else if trimmed.starts_with("## ") {
        Line::from(Span::styled(
            trimmed[3..].to_string(),
            Style::default().fg(theme.accent).add_modifier(Modifier::UNDERLINED),
        ))
    } else if trimmed.starts_with("### ") {
        Line::from(Span::styled(
            trimmed[4..].to_string(),
            Style::default().fg(theme.foreground).add_modifier(Modifier::BOLD),
        ))
    } else if trimmed.starts_with('*') || trimmed.starts_with('-') {
        let rest = trimmed[1..].trim().to_string();
        Line::from(vec![
            Span::styled("• ", Style::default().fg(theme.accent)),
            Span::raw(rest),
        ])
    } else if trimmed.starts_with("**") && trimmed.ends_with("**") && trimmed.len() > 4 {
        Line::from(Span::styled(
            trimmed[2..trimmed.len() - 2].to_string(),
            Style::default().fg(theme.warning).add_modifier(Modifier::BOLD),
        ))
    } else {
        Line::from(Span::raw(line.to_string()))
    }
}

fn draw_settings_screen(
    frame: &mut Frame,
    area: Rect,
    theme: &ThemeStyles,
    settings_focused_pane: usize,
    settings_selected_category: usize,
    settings_selected_option: usize,
    _scanned_fonts: &[String],
    config: &Config,
    status_msg: Option<&str>,
    settings_input_mode: bool,
    settings_input_buffer: &str,
) {
    let is_zh = config.general.language == "zh";
    let title_text = if is_zh {
        " 系统设置 [Tab/←/→: 切换面板 | F1/Esc: 返回] "
    } else {
        " Rune Settings [Tab/←/→: Switch Pane | F1/Esc: Return] "
    };

    let settings_block = Block::default()
        .title(Span::styled(
            title_text,
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.background).fg(theme.foreground));

    let inner_area = settings_block.inner(area);
    frame.render_widget(settings_block, area);

    // Split workspace into Category list (left 35%) and Option list (right 65%)
    let workspace = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(inner_area);

    let left_pane = workspace[0];
    let right_pane = workspace[1];

    // Left Category List (Expanded to 8 categories)
    let categories = if is_zh {
        vec![
            "1. 主题选择 (TUI Theme)",
            "2. 语言切换 (UI Language)",
            "3. 文本编辑器 (Text Editor)",
            "4. 插件管理 (Active Plugins)",
            "5. AI 提供商 (AI Provider)",
            "6. AI 参数配置 (AI Credentials)",
            "7. 文件搜索深度 (Files Max Depth)",
            "8. 关于项目 (About Rune)",
        ]
    } else {
        vec![
            "1. TUI Theme",
            "2. UI Language",
            "3. Text Editor",
            "4. Active Plugins",
            "5. AI Provider",
            "6. AI Credentials",
            "7. Files Max Depth",
            "8. About Rune",
        ]
    };

    let cat_items: Vec<ListItem> = categories
        .iter()
        .enumerate()
        .map(|(idx, cat)| {
            let is_hovered = idx == settings_selected_category;
            let is_focused = settings_focused_pane == 0 && is_hovered;
            
            let style = if is_focused {
                Style::default().bg(theme.selection).fg(theme.accent).add_modifier(Modifier::BOLD)
            } else if is_hovered {
                Style::default().bg(theme.selection).fg(theme.foreground)
            } else {
                Style::default().fg(theme.foreground)
            };
            
            let prefix = if is_hovered { "▶ " } else { "  " };
            ListItem::new(Line::from(Span::styled(format!("{prefix}{cat}"), style)))
        })
        .collect();

    let cat_title = if is_zh { " 配置分类 " } else { " Settings Categories " };
    let cat_border_style = if settings_focused_pane == 0 {
        Style::default().fg(theme.accent)
    } else {
        Style::default().fg(theme.border)
    };

    let cat_list = List::new(cat_items)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(cat_border_style)
            .title(cat_title));
    frame.render_widget(cat_list, left_pane);

    // Right Option Details Panel
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(65),
            Constraint::Percentage(25),
            Constraint::Percentage(10),
        ])
        .split(right_pane);

    let opt_pane = right_chunks[0];
    let desc_pane = right_chunks[1];
    let status_pane = right_chunks[2];

    let mut options = Vec::new();
    let mut active_val = String::new();
    let mut desc = String::new();
    
    match settings_selected_category {
        0 => {
            options = vec![
                "catppuccin".to_string(),
                "tokyo_night".to_string(),
                "nord".to_string(),
                "gruvbox".to_string(),
                "everforest".to_string(),
            ];
            active_val = config.theme.active.clone();
            desc = if is_zh {
                "选择主界面的着色主题。支持 Tokyo Night, Catppuccin, Nord 等配色，按 Enter 键可 live 实时切换预览。".to_string()
            } else {
                "Select UI color theme. Supports Tokyo Night, Catppuccin, Nord, etc. Press Enter to live preview and switch.".to_string()
            };
        }
        1 => {
            options = vec!["zh".to_string(), "en".to_string()];
            active_val = config.general.language.clone();
            desc = if is_zh {
                "切换主界面的语言。支持中文 (zh) 和英文 (en)，按 Enter 确定切换。".to_string()
            } else {
                "Switch UI display language. Supports Chinese (zh) and English (en). Press Enter to apply.".to_string()
            };
        }
        2 => {
            options = vec![
                "nano".to_string(),
                "vim".to_string(),
                "nvim".to_string(),
                "hx".to_string(),
                format!("custom ({})", config.general.editor),
            ];
            active_val = config.general.editor.clone();
            desc = if is_zh {
                "设置默认编辑器。选择最后一项按 Enter 可手动输入自定义的编辑器命令。".to_string()
            } else {
                "Configure default editor. Select the last option and press Enter to type a custom command.".to_string()
            };
        }
        3 => {
            options = vec![
                "applications".to_string(),
                "files".to_string(),
                "file_manager".to_string(),
                "commands".to_string(),
                "calculator".to_string(),
                "unit_converter".to_string(),
                "ssh".to_string(),
                "clipboard".to_string(),
                "git".to_string(),
                "docker".to_string(),
                "systemd".to_string(),
                "ai".to_string(),
                "process".to_string(),
                "network".to_string(),
            ];
            desc = if is_zh {
                "切换各个功能插件的启用状态。按 Enter 键可对当前选中的插件进行开启/关闭。禁用未使用的插件可优化响应速度。".to_string()
            } else {
                "Toggle activation status of specific functional plugins. Press Enter to enable/disable. Disabling unused plugins speeds up search.".to_string()
            };
        }
        4 => {
            options = vec!["openai".to_string(), "gemini".to_string(), "ollama".to_string()];
            active_val = config.plugins.ai_provider.clone();
            desc = if is_zh {
                "选择用于 AI 助手插件的底层服务模型提供商。可在 config.toml 中配置对应的 API Key。".to_string()
            } else {
                "Select AI model provider used by the AI chatbot plugin. Make sure to configure the API key in config.toml.".to_string()
            };
        }
        5 => {
            let masked_key = if config.plugins.ai_api_key.is_empty() {
                "[Not Set]".to_string()
            } else {
                let key_len = config.plugins.ai_api_key.len();
                format!("{}...", &config.plugins.ai_api_key[..std::cmp::min(5, key_len)])
            };
            options = vec![
                format!("API Key: {}", masked_key),
                format!("Model: {}", config.plugins.ai_model),
                format!("API URL: {}", config.plugins.ai_api_url),
            ];
            desc = if is_zh {
                "配置 AI 助手的连接参数。选中选项按 Enter 键进入下方文本输入框进行编辑。".to_string()
            } else {
                "Configure AI chatbot credentials. Select any item and press Enter to edit in the text prompt below.".to_string()
            };
        }
        6 => {
            options = vec![
                "2".to_string(),
                "3".to_string(),
                "4".to_string(),
                "5".to_string(),
                "6".to_string(),
            ];
            active_val = config.plugins.files_max_depth.to_string();
            desc = if is_zh {
                "配置文件搜索时深度优先遍历的最大目录级数。级别越深，扫描文件越全面，但也更耗费磁盘性能。".to_string()
            } else {
                "Configure maximum directory depth for file system scanning. Deeper scans find more files but consume more I/O.".to_string()
            };
        }
        7 => {
            options = if is_zh {
                vec![
                    "项目名称: Rune Launcher".to_string(),
                    "项目版本: v0.1.0 (Rust)".to_string(),
                    "核心协议: MIT License".to_string(),
                    "源码仓库: github.com/aimy1/Rune".to_string(),
                    "开发作者: aisaniya".to_string(),
                ]
            } else {
                vec![
                    "Project: Rune Launcher".to_string(),
                    "Version: v0.1.0 (Rust)".to_string(),
                    "License: MIT License".to_string(),
                    "Github: github.com/aimy1/Rune".to_string(),
                    "Author: aisaniya".to_string(),
                ]
            };
            desc = if is_zh {
                "Rune 是一款基于 Rust 编写的高性能 TUI 应用启动器与命令面板。支持模糊搜索，内置计算器、插件扩展、AI 聊天及配置管理。".to_string()
            } else {
                "Rune is a high-performance, fuzzy-search terminal launcher and command palette. Featuring calculators, clipboard managers, systemd monitoring, and built-in AI chatbots.".to_string()
            };
        }
        _ => {}
    }

    let opt_items: Vec<ListItem> = options
        .iter()
        .enumerate()
        .map(|(idx, opt)| {
            let is_hovered = idx == settings_selected_option;
            let is_focused = settings_focused_pane == 1 && is_hovered;
            
            let is_active = if settings_selected_category == 3 {
                match opt.as_str() {
                    "applications" => config.plugins.applications,
                    "files" => config.plugins.files,
                    "file_manager" => config.plugins.file_manager,
                    "commands" => config.plugins.commands,
                    "calculator" => config.plugins.calculator,
                    "unit_converter" => config.plugins.unit_converter,
                    "ssh" => config.plugins.ssh,
                    "clipboard" => config.plugins.clipboard,
                    "git" => config.plugins.git,
                    "docker" => config.plugins.docker,
                    "systemd" => config.plugins.systemd,
                    "ai" => config.plugins.ai,
                    "process" => config.plugins.process,
                    "network" => config.plugins.network,
                    _ => false,
                }
            } else if settings_selected_category == 5 || settings_selected_category == 7 {
                false
            } else if settings_selected_category == 2 {
                if idx < 4 {
                    opt == &active_val
                } else {
                    !["nano", "vim", "nvim", "hx"].contains(&active_val.as_str())
                }
            } else {
                opt == &active_val
            };
            
            let style = if settings_selected_category == 7 {
                Style::default().fg(theme.foreground)
            } else if is_focused {
                Style::default().bg(theme.selection).fg(theme.accent).add_modifier(Modifier::BOLD)
            } else if is_hovered {
                Style::default().bg(theme.selection).fg(theme.foreground)
            } else {
                Style::default().fg(theme.foreground)
            };
            
            let prefix = if settings_selected_category == 7 {
                ""
            } else if is_hovered {
                "▶ "
            } else {
                "  "
            };
            let checked = if settings_selected_category == 7 {
                ""
            } else if is_active {
                " ✔ "
            } else {
                "   "
            };
            
            ListItem::new(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(checked, Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
                Span::styled(opt, style),
            ]))
        })
        .collect();

    let opt_title = if is_zh { " 配置选项 " } else { " Available Options " };
    let opt_border_style = if settings_focused_pane == 1 {
        Style::default().fg(theme.accent)
    } else {
        Style::default().fg(theme.border)
    };

    let opt_list = List::new(opt_items)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(opt_border_style)
            .title(opt_title));
    frame.render_widget(opt_list, opt_pane);

    let desc_title = if is_zh { " 配置说明 " } else { " Description " };
    let desc_p = Paragraph::new(desc)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(theme.border)).title(desc_title))
        .style(Style::default().fg(theme.foreground))
        .wrap(ratatui::widgets::Wrap { trim: true });
    frame.render_widget(desc_p, desc_pane);

    // Status / Input text block
    let status_text = if settings_input_mode {
        let field_label = match settings_selected_category {
            2 => if is_zh { "编辑编辑器命令: " } else { "Edit Editor Command: " },
            5 => match settings_selected_option {
                0 => if is_zh { "编辑 API Key: " } else { "Edit API Key: " },
                1 => if is_zh { "编辑模型名称 (Model): " } else { "Edit Model: " },
                2 => if is_zh { "编辑 API URL 终结点: " } else { "Edit API URL: " },
                _ => "",
            },
            _ => "",
        };
        
        Line::from(vec![
            Span::styled(field_label, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(settings_input_buffer, Style::default().fg(theme.foreground)),
            Span::styled("█", Style::default().fg(theme.accent)),
            Span::raw(if is_zh { " [Enter 确认 │ Esc 取消]" } else { " [Enter Save │ Esc Cancel]" }),
        ])
    } else if let Some(msg) = status_msg {
        Line::from(Span::styled(msg, Style::default().fg(theme.success).add_modifier(Modifier::BOLD)))
    } else {
        Line::from(Span::styled(
            if is_zh { "使用 ↑/↓ 移动选择，Enter 确认修改/编辑，Esc/F1 关闭" } else { "Use ↑/↓ to navigate, Enter to select/edit, Esc/F1 to close" },
            Style::default().fg(theme.border),
        ))
    };
    
    let status_p = Paragraph::new(status_text)
        .style(Style::default().bg(theme.background));
    frame.render_widget(status_p, status_pane);
}

fn format_breadcrumbs(path_str: &str, is_zh: bool) -> String {
    let path = Path::new(path_str);
    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/home/fd"));
    
    let mut display_str = String::new();
    if path.starts_with(&home) {
        display_str.push_str(if is_zh { " 🏠 个人目录" } else { " 🏠 ~" });
        if let Ok(suffix) = path.strip_prefix(&home) {
            for component in suffix.components() {
                display_str.push_str(" > 📂 ");
                display_str.push_str(&component.as_os_str().to_string_lossy());
            }
        }
    } else {
        display_str.push_str(" 📁 /");
        for component in path.components() {
            let name = component.as_os_str().to_string_lossy();
            if name != "/" && !name.is_empty() {
                display_str.push_str(" > 📂 ");
                display_str.push_str(&name);
            }
        }
    }
    display_str
}

pub fn draw_app(
    frame: &mut Frame,
    query: &str,
    active_plugin_name: &str,
    results: &[SearchResult],
    list_state: &mut ListState,
    preview_content: Option<String>,
    preview_scroll: u16,
    theme: &ThemeStyles,
    status_msg: Option<&str>,
    total_plugins_count: usize,
    // Settings configuration
    settings_open: bool,
    settings_focused_pane: usize,
    settings_selected_category: usize,
    settings_selected_option: usize,
    scanned_fonts: &[String],
    config: &Config,
    settings_input_mode: bool,
    settings_input_buffer: &str,
    file_manager_open: bool,
) {
    let size = frame.size();

    // Fill screen with background
    let bg_block = Block::default().style(Style::default().bg(theme.background));
    frame.render_widget(bg_block, size);

    // Centered window for a modern look (Spotlight / Raycast style)
    let area = if size.width > 80 && size.height > 24 {
        centered_rect(80, 80, size)
    } else {
        size
    };

    frame.render_widget(Clear, area);

    if settings_open {
        draw_settings_screen(
            frame,
            area,
            theme,
            settings_focused_pane,
            settings_selected_category,
            settings_selected_option,
            scanned_fonts,
            config,
            status_msg,
            settings_input_mode,
            settings_input_buffer,
        );
        return;
    }

    let is_zh = config.general.language == "zh";

    if file_manager_open {
        let selected_idx = list_state.selected().unwrap_or(0);
        let first_res = results.iter().find(|r| r.plugin_id == "file_manager");
        
        let focus_pane = first_res
            .and_then(|r| r.metadata.get("focus_pane").and_then(|s| s.parse::<usize>().ok()))
            .unwrap_or(1);

        let current_dir_str = first_res
            .and_then(|r| r.metadata.get("current_dir").cloned())
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .unwrap_or_else(|_| dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/home/fd")))
                    .to_string_lossy()
                    .to_string()
            });

        let outer_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.background).fg(theme.foreground));
        let inner_area = outer_block.inner(area);
        frame.render_widget(outer_block, area);

        let file_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Top Bar (Header)
                Constraint::Min(4),    // Main workspace
                Constraint::Length(1), // Bottom Status Bar
            ])
            .split(inner_area);

        let top_area = file_chunks[0];
        let main_area = file_chunks[1];
        let footer_area = file_chunks[2];

        // 1. Render Top Header
        let header_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(10), Constraint::Length(35)])
            .split(top_area);

        // Location Path Address Bar (Left Header)
        let location_title = if is_zh { " 当前路径 " } else { " Location " };
        let path_block = Block::default()
            .borders(Borders::ALL)
            .border_style(if focus_pane == 2 {
                Style::default().fg(theme.accent)
            } else {
                Style::default().fg(theme.border)
            })
            .border_type(if focus_pane == 2 {
                ratatui::widgets::BorderType::Double
            } else {
                ratatui::widgets::BorderType::Plain
            })
            .title(location_title);

        if focus_pane == 2 {
            let path_p = Paragraph::new(Line::from(vec![
                Span::raw(" "),
                Span::styled(query, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            ])).block(path_block);
            frame.render_widget(path_p, header_chunks[0]);
        } else {
            let home_dir = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/home/fd"));
            let current_path = std::path::Path::new(&current_dir_str);
            
            let mut path_spans = Vec::new();
            path_spans.push(Span::raw(" "));
            if current_path.starts_with(&home_dir) {
                path_spans.push(Span::styled("🏠 Home", Style::default().fg(theme.accent)));
                if let Ok(suffix) = current_path.strip_prefix(&home_dir) {
                    for component in suffix.components() {
                        path_spans.push(Span::styled(" › ", Style::default().fg(theme.border).add_modifier(Modifier::DIM)));
                        path_spans.push(Span::styled(format!("📂 {}", component.as_os_str().to_string_lossy()), Style::default().fg(theme.foreground)));
                    }
                }
            } else {
                path_spans.push(Span::styled("📁 Root", Style::default().fg(theme.accent)));
                for component in current_path.components() {
                    let name = component.as_os_str().to_string_lossy();
                    if name != "/" && !name.is_empty() {
                        path_spans.push(Span::styled(" › ", Style::default().fg(theme.border).add_modifier(Modifier::DIM)));
                        path_spans.push(Span::styled(format!("📂 {}", name), Style::default().fg(theme.foreground)));
                    }
                }
            }
            
            if let Some(last) = path_spans.last_mut() {
                if last.content != " " {
                    last.style = Style::default().fg(theme.accent).add_modifier(Modifier::BOLD);
                }
            }

            let path_p = Paragraph::new(Line::from(path_spans)).block(path_block);
            frame.render_widget(path_p, header_chunks[0]);
        }

        // Search Query input (Right Header)
        let search_title = if is_zh { " 搜索过滤 " } else { " Search Query " };
        let search_block = Block::default()
            .title(Span::styled(search_title, Style::default().fg(theme.border)))
            .borders(Borders::ALL)
            .border_style(if focus_pane == 3 {
                Style::default().fg(theme.accent)
            } else {
                Style::default().fg(theme.border)
            })
            .border_type(if focus_pane == 3 {
                ratatui::widgets::BorderType::Double
            } else {
                ratatui::widgets::BorderType::Plain
            });
        let search_display_text = if focus_pane == 2 { "" } else { query };
        let cursor_span = Span::raw(search_display_text);
        let search_p = Paragraph::new(Line::from(vec![
            Span::styled("🔍 ", Style::default().fg(theme.accent)),
            cursor_span,
        ]))
        .block(search_block)
        .style(Style::default().bg(theme.background));
        frame.render_widget(search_p, header_chunks[1]);

        // Place terminal cursor dynamically in the correct focused input box (offset: 1 border + 3/1 padding)
        if focus_pane == 2 {
            let cursor_x = (header_chunks[0].x + 2 + query.chars().count() as u16)
                .min(header_chunks[0].x + header_chunks[0].width.saturating_sub(2));
            let cursor_y = header_chunks[0].y + 1;
            frame.set_cursor(cursor_x, cursor_y);
        } else if focus_pane == 3 {
            let cursor_x = (header_chunks[1].x + 4 + query.chars().count() as u16)
                .min(header_chunks[1].x + header_chunks[1].width.saturating_sub(2));
            let cursor_y = header_chunks[1].y + 1;
            frame.set_cursor(cursor_x, cursor_y);
        }

        // 2. Split Main Workspace into Sidebar and Files list
        let main_columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(24), Constraint::Min(20)])
            .split(main_area);

        let sidebar_area = main_columns[0];
        let files_area = main_columns[1];

        // Split Sidebar vertically into Favorites, Preview, and Storage
        let sidebar_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8), // Favorites List
                Constraint::Min(5),    // Selected item preview metadata
                Constraint::Length(3), // Disk Storage Capacity
            ])
            .split(sidebar_area);

        let fav_area = sidebar_rows[0];
        let prev_area = sidebar_rows[1];
        let disk_area = sidebar_rows[2];

        let first_res = results.iter().find(|r| r.plugin_id == "file_manager");
        let sidebar_focused = first_res
            .and_then(|r| r.metadata.get("sidebar_focused").map(|s| s == "true"))
            .unwrap_or(false);
        let sidebar_selected_idx = first_res
            .and_then(|r| r.metadata.get("sidebar_selected_idx").and_then(|s| s.parse::<usize>().ok()))
            .unwrap_or(0);

        // --- Render Favorites ---
        let home_c = first_res.and_then(|r| r.metadata.get("fav_home_count").map(|s| s.as_str())).unwrap_or("0");
        let docs_c = first_res.and_then(|r| r.metadata.get("fav_docs_count").map(|s| s.as_str())).unwrap_or("0");
        let pics_c = first_res.and_then(|r| r.metadata.get("fav_pics_count").map(|s| s.as_str())).unwrap_or("0");
        let music_c = first_res.and_then(|r| r.metadata.get("fav_music_count").map(|s| s.as_str())).unwrap_or("0");
        let downloads_c = first_res.and_then(|r| r.metadata.get("fav_downloads_count").map(|s| s.as_str())).unwrap_or("0");

        let fav_items = vec![
            (format!("🏠 Home ({})", home_c), 0),
            (format!("📄 Documents ({})", docs_c), 1),
            (format!("📷 Pictures ({})", pics_c), 2),
            (format!("🎵 Music ({})", music_c), 3),
            (format!("📦 Downloads ({})", downloads_c), 4),
        ];

        let fav_title = if is_zh { " 收藏夹 " } else { " Favorites " };
        let fav_block = Block::default()
            .borders(Borders::ALL)
            .border_style(if sidebar_focused {
                Style::default().fg(theme.accent)
            } else {
                Style::default().fg(theme.border)
            })
            .border_type(if sidebar_focused {
                ratatui::widgets::BorderType::Double
            } else {
                ratatui::widgets::BorderType::Plain
            })
            .title(fav_title);

        let list_items: Vec<ListItem> = fav_items
            .iter()
            .map(|(label, idx)| {
                let is_selected = *idx == sidebar_selected_idx;
                let style = if is_selected {
                    if sidebar_focused {
                        Style::default().bg(theme.selection).fg(theme.accent).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().bg(theme.selection).fg(theme.foreground)
                    }
                } else {
                    Style::default().fg(theme.foreground)
                };
                let prefix = if is_selected { "▶ " } else { "  " };
                ListItem::new(Line::from(Span::styled(format!("{prefix}{label}"), style)))
            })
            .collect();

        let fav_list = List::new(list_items)
            .block(fav_block)
            .style(Style::default().bg(theme.background));
        frame.render_widget(fav_list, fav_area);

        // --- Render Preview (bottom-left) ---
        let preview_title = if is_zh { " 预览面板 " } else { " Preview " };
        let prev_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title(preview_title);

        let preview_lines = if let Some(ref text) = preview_content {
            let mut lines = Vec::new();
            let mut in_code_block = false;
            for line in text.lines() {
                let trimmed = line.trim();
                if line.starts_with("[IMAGE: ") && line.ends_with(']') {
                    let path_str = &line[8..line.len() - 1];
                    lines.extend(draw_image_in_preview(path_str, theme, 20, 10));
                    continue;
                }
                if trimmed.starts_with("```") {
                    in_code_block = !in_code_block;
                    continue;
                }
                if trimmed.starts_with("---") {
                    break;
                }
                if in_code_block {
                    lines.push(Line::from(Span::styled(line.to_string(), Style::default().fg(theme.success))));
                } else if trimmed.starts_with('|') {
                    if trimmed.contains("---") {
                        continue;
                    }
                    let parts: Vec<&str> = trimmed.split('|').collect();
                    if parts.len() >= 4 {
                        let raw_key = parts[1].trim();
                        let raw_val = parts[2].trim();
                        if raw_key == "Metadata" || raw_key.is_empty() {
                            continue;
                        }
                        let key = raw_key.replace("**", "").replace("`", "");
                        let val = raw_val.replace("**", "").replace("`", "");
                        
                        let display_key = if is_zh {
                            match key.as_str() {
                                "Type" => "类型".to_string(),
                                "Size" => "大小".to_string(),
                                "Permissions" => "权限".to_string(),
                                "Owner" => "所有者".to_string(),
                                "Modified" => "修改时间".to_string(),
                                _ => key,
                            }
                        } else {
                            key
                        };

                        lines.push(Line::from(vec![
                            Span::styled(format!(" {:<10}: ", display_key), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                            Span::styled(val, Style::default().fg(theme.foreground)),
                        ]));
                    }
                } else if trimmed.starts_with("# ") {
                    lines.push(Line::from(Span::styled(
                        format!(" {}", trimmed[2..].to_string()),
                        Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)
                    )));
                } else if trimmed.starts_with("*Path:") {
                    let path_val = trimmed.trim_start_matches("*Path:").trim_end_matches('*').trim();
                    lines.push(Line::from(vec![
                        Span::styled(" Path: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                        Span::styled(path_val.to_string(), Style::default().fg(theme.border)),
                    ]));
                    lines.push(Line::from(""));
                } else {
                    lines.push(parse_markdown_line(line, theme));
                }
            }
            lines
        } else {
            vec![Line::from(Span::styled(
                if is_zh { " (无选中文件) " } else { " (No selection) " },
                Style::default().fg(theme.border)
            ))]
        };

        let preview_p = Paragraph::new(preview_lines)
            .block(prev_block)
            .style(Style::default().bg(theme.background))
            .wrap(ratatui::widgets::Wrap { trim: true });
        frame.render_widget(preview_p, prev_area);

        // --- Render Storage Capacity Bar ---
        let disk_total = first_res.and_then(|r| r.metadata.get("disk_total").map(|s| s.as_str())).unwrap_or("-");
        let disk_used = first_res.and_then(|r| r.metadata.get("disk_used").map(|s| s.as_str())).unwrap_or("-");
        let disk_percent = first_res.and_then(|r| r.metadata.get("disk_percent").map(|s| s.as_str())).unwrap_or("-%");

        let pct_val = disk_percent.trim_end_matches('%').parse::<u8>().unwrap_or(0);
        let bar_width = 10;
        let filled = ((pct_val as f32 / 100.0) * bar_width as f32).round() as usize;
        let bar_str = format!(
            " [{}{}] {} ",
            "█".repeat(filled),
            "░".repeat(bar_width - filled),
            disk_percent
        );
        let disk_detail = format!(" {} / {}", disk_used, disk_total);

        let disk_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title(if is_zh { " 💾 存储空间 " } else { " 💾 Storage " });
        let disk_p = Paragraph::new(vec![
            Line::from(Span::styled(bar_str, Style::default().fg(if pct_val > 85 { theme.error } else { theme.accent }).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled(disk_detail, Style::default().fg(theme.border).add_modifier(Modifier::DIM))),
        ]).block(disk_block);
        frame.render_widget(disk_p, disk_area);

        // --- Render Right Column (Breadcrumbs & Files List) ---
        let files_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Column headers
                Constraint::Min(1),    // Files List
            ])
            .split(files_area);

        let header_area = files_rows[0];
        let list_area = files_rows[1];

        // Column Headers (Responsive based on width)
        let width = files_area.width as usize;
        let size_w = 10;
        let perm_w = 11;
        let mod_w = 16;

        let (show_perm, show_mod) = if width > 75 {
            (true, true)
        } else if width > 55 {
            (false, true)
        } else {
            (false, false)
        };

        let name_w = if show_perm && show_mod {
            width.saturating_sub(12 + size_w + perm_w + mod_w)
        } else if show_mod {
            width.saturating_sub(8 + size_w + mod_w)
        } else {
            width.saturating_sub(6 + size_w)
        };

        let mut header_spans = vec![
            Span::raw("   "), // Align headers with item prefix inside borders
            Span::styled(format!("{:<width$}", if is_zh { "名称" } else { "Name" }, width = name_w), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled(format!("{:>width$}", if is_zh { "大小" } else { "Size" }, width = size_w), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        ];

        if show_perm {
            header_spans.push(Span::raw("  "));
            header_spans.push(Span::styled(format!("{:>width$}", if is_zh { "权限" } else { "Permissions" }, width = perm_w), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)));
        }

        if show_mod {
            header_spans.push(Span::raw("  "));
            header_spans.push(Span::styled(format!("{:>width$}", if is_zh { "修改时间" } else { "Modified" }, width = mod_w), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)));
        }

        let header_p = Paragraph::new(Line::from(header_spans));
        frame.render_widget(header_p, header_area);

        // Files List (Redesigned as an aligned multi-column responsive list)
        let files_border_style = if focus_pane == 1 {
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.border)
        };

        let actual_results_count = results.iter().filter(|r| r.id != "dummy_metadata_carrier").count();

        let matches_title = if is_zh {
            format!(" 文件列表 (共 {} 项) ", actual_results_count)
        } else {
            format!(" Files List ({} items) ", actual_results_count)
        };

        let items_list: Vec<ListItem> = results
            .iter()
            .filter(|r| r.id != "dummy_metadata_carrier")
            .enumerate()
            .map(|(idx, res)| {
                let is_selected = idx == selected_idx;
                let style = if is_selected {
                    Style::default().bg(theme.selection)
                } else {
                    Style::default()
                };

                let prefix = if is_selected && focus_pane == 1 { "▶ " } else { "  " };

                let name = res.metadata.get("name").cloned().unwrap_or_else(|| res.title.clone());
                let icon = res.metadata.get("icon").map(|s| s.as_str()).unwrap_or("📄");
                let size = res.metadata.get("size").cloned().unwrap_or_default();
                let modified = res.metadata.get("modified").cloned().unwrap_or_default();
                let permissions = res.metadata.get("permissions").cloned().unwrap_or_else(|| "---------".to_string());

                let clip_path = res.metadata.get("clip_path").map(|s| s.as_str()).unwrap_or("");
                let clip_op = res.metadata.get("clip_op").map(|s| s.as_str()).unwrap_or("");
                let is_cut = clip_op == "Cut" && clip_path == res.id;

                let mut name_part = format!("{icon} {name}");
                if name_part.len() > name_w {
                    name_part.truncate(name_w.saturating_sub(3));
                    name_part.push_str("...");
                }

                let name_padded = format!("{:<width$}", name_part, width = name_w);
                let size_padded = format!("{:>width$}", size, width = size_w);

                let name_style = if is_selected && focus_pane == 1 {
                    Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)
                } else if is_cut {
                    Style::default().fg(theme.border).add_modifier(Modifier::DIM)
                } else {
                    Style::default().fg(theme.foreground)
                };

                let size_style = Style::default().fg(if is_selected { theme.foreground } else { theme.border });

                let mut item_spans = vec![
                    Span::styled(prefix, Style::default().fg(theme.accent)),
                    Span::styled(name_padded, name_style),
                    Span::raw("  "),
                    Span::styled(size_padded, size_style),
                ];

                if show_perm {
                    let perm_padded = format!("{:>width$}", permissions, width = perm_w);
                    let perm_style = Style::default().fg(if is_selected { theme.foreground } else { theme.border }).add_modifier(Modifier::DIM);
                    item_spans.push(Span::raw("  "));
                    item_spans.push(Span::styled(perm_padded, perm_style));
                }

                if show_mod {
                    let mod_padded = format!("{:>width$}", modified, width = mod_w);
                    let mod_style = Style::default().fg(if is_selected { theme.foreground } else { theme.border });
                    item_spans.push(Span::raw("  "));
                    item_spans.push(Span::styled(mod_padded, mod_style));
                }

                ListItem::new(Line::from(item_spans)).style(style)
            })
            .collect();

        let files_block = Block::default()
            .borders(Borders::ALL)
            .border_style(files_border_style)
            .border_type(if focus_pane == 1 {
                ratatui::widgets::BorderType::Double
            } else {
                ratatui::widgets::BorderType::Plain
            })
            .title(matches_title);

        if actual_results_count == 0 {
            let empty_text = if is_zh {
                "\n\n\n   (空文件夹)"
            } else {
                "\n\n\n   (Empty Directory)"
            };
            let empty_p = Paragraph::new(Span::styled(empty_text, Style::default().fg(theme.border).add_modifier(Modifier::ITALIC)))
                .block(files_block)
                .style(Style::default().bg(theme.background));
            frame.render_widget(empty_p, list_area);
        } else {
            let list_widget = List::new(items_list)
                .block(files_block)
                .style(Style::default().bg(theme.background));
            frame.render_stateful_widget(list_widget, list_area, list_state);
        }

        // --- Render Footer ---
        let total_dirs = first_res.and_then(|r| r.metadata.get("total_dirs").map(|s| s.as_str())).unwrap_or("0");
        let total_files = first_res.and_then(|r| r.metadata.get("total_files").map(|s| s.as_str())).unwrap_or("0");

        let selected_desc = if !results.is_empty() && selected_idx < results.len() {
            let sel = &results[selected_idx];
            let name = sel.metadata.get("name").map(|s| s.as_str()).unwrap_or("");
            let size = sel.metadata.get("size").map(|s| s.as_str()).unwrap_or("");
            if name == ".." {
                if is_zh { "返回上一级".to_string() } else { "Go Up".to_string() }
            } else {
                format!("{name} ({size})")
            }
        } else {
            if is_zh { "无选中".to_string() } else { "No selection".to_string() }
        };

        let stats_str = if is_zh {
            format!(" 📁 {total_dirs} 个文件夹, 📄 {total_files} 个文件 │ 当前选中: {selected_desc} ")
        } else {
            format!(" 📁 {total_dirs} folders, 📄 {total_files} files │ Selected: {selected_desc} ")
        };

        let footer_text = if let Some(msg) = status_msg {
            Span::styled(msg, Style::default().fg(theme.success).add_modifier(Modifier::BOLD))
        } else {
            Span::styled(stats_str, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
        };

        let cheatsheet_text = if is_zh {
            " ⌨️ F2:关闭 | ←/→:切换焦点 | Alt-N:建文件夹 | Alt-F:建文件 | Alt-R:重命名 | Delete:删除 | F4:打开终端 "
        } else {
            " ⌨️ F2:Close | ←/→:Focus | Alt-N:Dir | Alt-F:File | Alt-R:Rename | Del:Delete | F4:Terminal "
        };

        let footer_p = Paragraph::new(Line::from(vec![
            footer_text,
            Span::raw(" | "),
            Span::styled(cheatsheet_text, Style::default().fg(theme.border).add_modifier(Modifier::DIM)),
        ]))
        .style(Style::default().bg(theme.background));
        frame.render_widget(footer_p, footer_area);

        return;
    }

    // Main window outline container block
    let outer_title = if is_zh {
        format!(" Rune 启动器 [{active_plugin_name}] ")
    } else {
        format!(" Rune Launcher [{active_plugin_name}] ")
    };

    let outer_block = Block::default()
        .title(Span::styled(
            outer_title,
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.background).fg(theme.foreground));

    let inner_area = outer_block.inner(area);
    frame.render_widget(outer_block, area);

    // Division of main client workspace area
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search Input box
            Constraint::Min(4),    // Search results + preview window
            Constraint::Length(1), // Footer keys
        ])
        .split(inner_area);

    // 1. Render Search Box
    let search_title = if is_zh { " 搜索输入 " } else { " Search Query " };
    let search_block = Block::default()
        .title(Span::styled(search_title, Style::default().fg(theme.border)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border));

    // Show text cursor at end of query
    let cursor_span = Span::raw(query);
    let search_p = Paragraph::new(Line::from(vec![
        Span::styled("🔍 ", Style::default().fg(theme.accent)),
        cursor_span,
    ]))
    .block(search_block)
    .style(Style::default().bg(theme.background));

    frame.render_widget(search_p, chunks[0]);

    // Place terminal cursor in the search input box (offset: 1 border + 3 for "🔍 " emoji)
    let cursor_x = (chunks[0].x + 4 + query.chars().count() as u16)
        .min(chunks[0].x + chunks[0].width.saturating_sub(2));
    let cursor_y = chunks[0].y + 1;
    frame.set_cursor(cursor_x, cursor_y);

    // 2. Render Results and Preview
    let middle_area = chunks[1];
    
    // Determine layout depending on preview existence
    let (left_area, right_area) = if let Some(ref preview_text) = preview_content {
        let split_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(middle_area);
        (split_chunks[0], Some((split_chunks[1], preview_text)))
    } else {
        (middle_area, None)
    };

    let selected_idx = list_state.selected().unwrap_or(0);
    let is_file_manager = false;

    if is_file_manager {
        let outer_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.background).fg(theme.foreground));
        let inner_area = outer_block.inner(area);
        frame.render_widget(outer_block, area);

        let file_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Top Bar (Header)
                Constraint::Min(4),    // Main workspace
                Constraint::Length(1), // Bottom Status Bar
            ])
            .split(inner_area);

        let top_area = file_chunks[0];
        let main_area = file_chunks[1];
        let footer_area = file_chunks[2];

        // 1. Render Top Header
        let header_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(10), Constraint::Length(35)])
            .split(top_area);

        let title_p = Paragraph::new(Line::from(vec![
            Span::styled(" ᚱ ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("Rune Files", Style::default().fg(theme.foreground).add_modifier(Modifier::BOLD)),
        ])).style(Style::default().bg(theme.background));
        frame.render_widget(title_p, header_chunks[0]);

        let search_title = if is_zh { " 搜索过滤 " } else { " Search Query " };
        let search_block = Block::default()
            .title(Span::styled(search_title, Style::default().fg(theme.border)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border));
        let cursor_span = Span::raw(query);
        let search_p = Paragraph::new(Line::from(vec![
            Span::styled("🔍 ", Style::default().fg(theme.accent)),
            cursor_span,
        ]))
        .block(search_block)
        .style(Style::default().bg(theme.background));
        frame.render_widget(search_p, header_chunks[1]);

        // 2. Split Main Workspace into Sidebar and Files list
        let main_columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(24), Constraint::Min(20)])
            .split(main_area);

        let sidebar_area = main_columns[0];
        let files_area = main_columns[1];

        // Split Sidebar vertically into Favorites, Preview, and Storage
        let sidebar_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8), // Favorites List
                Constraint::Min(5),    // Selected item preview metadata
                Constraint::Length(3), // Disk Storage Capacity
            ])
            .split(sidebar_area);

        let fav_area = sidebar_rows[0];
        let prev_area = sidebar_rows[1];
        let disk_area = sidebar_rows[2];

        let first_res = results.iter().find(|r| r.plugin_id == "file_manager");
        let sidebar_focused = first_res
            .and_then(|r| r.metadata.get("sidebar_focused").map(|s| s == "true"))
            .unwrap_or(false);
        let sidebar_selected_idx = first_res
            .and_then(|r| r.metadata.get("sidebar_selected_idx").and_then(|s| s.parse::<usize>().ok()))
            .unwrap_or(0);

        // --- Render Favorites ---
        let home_c = first_res.and_then(|r| r.metadata.get("fav_home_count").map(|s| s.as_str())).unwrap_or("0");
        let docs_c = first_res.and_then(|r| r.metadata.get("fav_docs_count").map(|s| s.as_str())).unwrap_or("0");
        let pics_c = first_res.and_then(|r| r.metadata.get("fav_pics_count").map(|s| s.as_str())).unwrap_or("0");
        let music_c = first_res.and_then(|r| r.metadata.get("fav_music_count").map(|s| s.as_str())).unwrap_or("0");
        let downloads_c = first_res.and_then(|r| r.metadata.get("fav_downloads_count").map(|s| s.as_str())).unwrap_or("0");

        let fav_items = vec![
            (format!("🏠 Home ({})", home_c), 0),
            (format!("📄 Documents ({})", docs_c), 1),
            (format!("📷 Pictures ({})", pics_c), 2),
            (format!("🎵 Music ({})", music_c), 3),
            (format!("📦 Downloads ({})", downloads_c), 4),
        ];

        let fav_title = if is_zh { " 收藏夹 " } else { " Favorites " };
        let fav_block = Block::default()
            .borders(Borders::ALL)
            .border_style(if sidebar_focused {
                Style::default().fg(theme.accent)
            } else {
                Style::default().fg(theme.border)
            })
            .border_type(if sidebar_focused {
                ratatui::widgets::BorderType::Double
            } else {
                ratatui::widgets::BorderType::Plain
            })
            .title(fav_title);

        let list_items: Vec<ListItem> = fav_items
            .iter()
            .map(|(label, idx)| {
                let is_selected = *idx == sidebar_selected_idx;
                let style = if is_selected {
                    if sidebar_focused {
                        Style::default().bg(theme.selection).fg(theme.accent).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().bg(theme.selection).fg(theme.foreground)
                    }
                } else {
                    Style::default().fg(theme.foreground)
                };
                let prefix = if is_selected { "▶ " } else { "  " };
                ListItem::new(Line::from(Span::styled(format!("{prefix}{label}"), style)))
            })
            .collect();

        let fav_list = List::new(list_items)
            .block(fav_block)
            .style(Style::default().bg(theme.background));
        frame.render_widget(fav_list, fav_area);

        // --- Render Preview (bottom-left) ---
        let preview_title = if is_zh { " 预览面板 " } else { " Preview " };
        let prev_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title(preview_title);

        let preview_lines = if let Some(ref text) = preview_content {
            let mut lines = Vec::new();
            let mut in_code_block = false;
            for line in text.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("```") {
                    in_code_block = !in_code_block;
                    continue;
                }
                if trimmed.starts_with("---") {
                    break;
                }
                if in_code_block {
                    lines.push(Line::from(Span::styled(line.to_string(), Style::default().fg(theme.success))));
                } else if trimmed.starts_with('|') {
                    if trimmed.contains("---") {
                        continue;
                    }
                    let parts: Vec<&str> = trimmed.split('|').collect();
                    if parts.len() >= 4 {
                        let raw_key = parts[1].trim();
                        let raw_val = parts[2].trim();
                        if raw_key == "Metadata" || raw_key.is_empty() {
                            continue;
                        }
                        let key = raw_key.replace("**", "").replace("`", "");
                        let val = raw_val.replace("**", "").replace("`", "");
                        
                        let display_key = if is_zh {
                            match key.as_str() {
                                "Type" => "类型".to_string(),
                                "Size" => "大小".to_string(),
                                "Permissions" => "权限".to_string(),
                                "Owner" => "所有者".to_string(),
                                _ => key,
                            }
                        } else {
                            key
                        };

                        lines.push(Line::from(vec![
                            Span::styled(format!(" {:<5}: ", display_key), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                            Span::styled(val, Style::default().fg(theme.foreground)),
                        ]));
                    }
                } else if trimmed.starts_with("# ") {
                    lines.push(Line::from(Span::styled(
                        format!(" {}", trimmed[2..].to_string()),
                        Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)
                    )));
                } else if trimmed.starts_with("*Path:") {
                    continue;
                } else {
                    lines.push(parse_markdown_line(line, theme));
                }
            }
            lines
        } else {
            vec![Line::from(Span::styled(
                if is_zh { " (无选中文件) " } else { " (No selection) " },
                Style::default().fg(theme.border)
            ))]
        };

        let preview_p = Paragraph::new(preview_lines)
            .block(prev_block)
            .style(Style::default().bg(theme.background))
            .wrap(ratatui::widgets::Wrap { trim: true });
        frame.render_widget(preview_p, prev_area);

        // --- Render Storage Capacity Bar ---
        let disk_total = first_res.and_then(|r| r.metadata.get("disk_total").map(|s| s.as_str())).unwrap_or("-");
        let disk_used = first_res.and_then(|r| r.metadata.get("disk_used").map(|s| s.as_str())).unwrap_or("-");
        let disk_percent = first_res.and_then(|r| r.metadata.get("disk_percent").map(|s| s.as_str())).unwrap_or("-%");

        let pct_val = disk_percent.trim_end_matches('%').parse::<u8>().unwrap_or(0);
        let bar_width = 10;
        let filled = ((pct_val as f32 / 100.0) * bar_width as f32).round() as usize;
        let bar_str = format!(
            " [{}{}] {} ",
            "█".repeat(filled),
            "░".repeat(bar_width - filled),
            disk_percent
        );
        let disk_detail = format!(" {} / {}", disk_used, disk_total);

        let disk_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title(if is_zh { " 💾 存储空间 " } else { " 💾 Storage " });
        let disk_p = Paragraph::new(vec![
            Line::from(Span::styled(bar_str, Style::default().fg(if pct_val > 85 { theme.error } else { theme.accent }).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled(disk_detail, Style::default().fg(theme.border).add_modifier(Modifier::DIM))),
        ]).block(disk_block);
        frame.render_widget(disk_p, disk_area);

        // --- Render Right Column (Breadcrumbs & Files List) ---
        let files_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Location / Breadcrumbs
                Constraint::Length(1), // Column headers
                Constraint::Min(1),    // Files List
            ])
            .split(files_area);

        let path_area = files_rows[0];
        let header_area = files_rows[1];
        let list_area = files_rows[2];

        let location_title = if is_zh { " 当前路径 " } else { " Location " };
        let path_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title(location_title);

        let current_dir_str = first_res
            .and_then(|r| r.metadata.get("current_dir").cloned())
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .unwrap_or_else(|_| dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/home/fd")))
                    .to_string_lossy()
                    .to_string()
            });

        // Parse beautiful breadcrumbs
        let home_dir = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/home/fd"));
        let current_path = std::path::Path::new(&current_dir_str);
        
        let mut path_spans = Vec::new();
        path_spans.push(Span::raw(" "));
        if current_path.starts_with(&home_dir) {
            path_spans.push(Span::styled("🏠 Home", Style::default().fg(theme.accent)));
            if let Ok(suffix) = current_path.strip_prefix(&home_dir) {
                for component in suffix.components() {
                    path_spans.push(Span::styled(" › ", Style::default().fg(theme.border).add_modifier(Modifier::DIM)));
                    path_spans.push(Span::styled(format!("📂 {}", component.as_os_str().to_string_lossy()), Style::default().fg(theme.foreground)));
                }
            }
        } else {
            path_spans.push(Span::styled("📁 Root", Style::default().fg(theme.accent)));
            for component in current_path.components() {
                let name = component.as_os_str().to_string_lossy();
                if name != "/" && !name.is_empty() {
                    path_spans.push(Span::styled(" › ", Style::default().fg(theme.border).add_modifier(Modifier::DIM)));
                    path_spans.push(Span::styled(format!("📂 {}", name), Style::default().fg(theme.foreground)));
                }
            }
        }
        
        if let Some(last) = path_spans.last_mut() {
            if last.content != " " {
                last.style = Style::default().fg(theme.accent).add_modifier(Modifier::BOLD);
            }
        }

        let path_p = Paragraph::new(Line::from(path_spans)).block(path_block);
        frame.render_widget(path_p, path_area);

        // Column Headers
        let avail_w = (files_area.width as usize).saturating_sub(6);
        let mod_w = 16;
        let perm_w = 11;
        let size_w = 10;
        let name_w = avail_w.saturating_sub(mod_w + size_w + perm_w + 6);

        let h_name = format!("{:<width$}", if is_zh { "名称" } else { "Name" }, width = name_w);
        let h_size = format!("{:>width$}", if is_zh { "大小" } else { "Size" }, width = size_w);
        let h_perm = format!("{:>width$}", if is_zh { "权限" } else { "Permissions" }, width = perm_w);
        let h_mod = format!("{:>width$}", if is_zh { "修改时间" } else { "Modified" }, width = mod_w);

        let header_line = Line::from(vec![
            Span::raw("  "),
            Span::styled(h_name, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled(h_size, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled(h_perm, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled(h_mod, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        ]);
        let header_p = Paragraph::new(header_line);
        frame.render_widget(header_p, header_area);

        // Files List (Redesigned as an aligned multi-column visual list)
        let files_border_style = if !sidebar_focused {
            Style::default().fg(theme.accent)
        } else {
            Style::default().fg(theme.border)
        };

        let matches_title = if is_zh {
            format!(" 文件列表 (共 {} 项) ", results.len())
        } else {
            format!(" Files List ({} items) ", results.len())
        };

        let items_list: Vec<ListItem> = results
            .iter()
            .enumerate()
            .map(|(idx, res)| {
                let is_selected = idx == selected_idx;
                let style = if is_selected {
                    Style::default().bg(theme.selection)
                } else {
                    Style::default()
                };

                let prefix = if is_selected && !sidebar_focused { "▶ " } else { "  " };

                let name = res.metadata.get("name").cloned().unwrap_or_else(|| res.title.clone());
                let icon = res.metadata.get("icon").map(|s| s.as_str()).unwrap_or("📄");
                let size = res.metadata.get("size").cloned().unwrap_or_default();
                let modified = res.metadata.get("modified").cloned().unwrap_or_default();
                let permissions = res.metadata.get("permissions").cloned().unwrap_or_else(|| "---------".to_string());

                let mut name_part = format!("{icon} {name}");
                if name_part.len() > name_w {
                    name_part.truncate(name_w.saturating_sub(3));
                    name_part.push_str("...");
                }

                let name_padded = format!("{:<width$}", name_part, width = name_w);
                let size_padded = format!("{:>width$}", size, width = size_w);
                let perm_padded = format!("{:>width$}", permissions, width = perm_w);
                let mod_padded = format!("{:>width$}", modified, width = mod_w);

                let name_style = if is_selected && !sidebar_focused {
                    Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.foreground)
                };

                let size_style = Style::default().fg(if is_selected { theme.foreground } else { theme.border });
                let perm_style = Style::default().fg(if is_selected { theme.foreground } else { theme.border }).add_modifier(Modifier::DIM);
                let mod_style = Style::default().fg(if is_selected { theme.foreground } else { theme.border });

                let line = Line::from(vec![
                    Span::styled(prefix, Style::default().fg(theme.accent)),
                    Span::styled(name_padded, name_style),
                    Span::raw("  "),
                    Span::styled(size_padded, size_style),
                    Span::raw("  "),
                    Span::styled(perm_padded, perm_style),
                    Span::raw("  "),
                    Span::styled(mod_padded, mod_style),
                ]);

                ListItem::new(line).style(style)
            })
            .collect();

        let files_block = Block::default()
            .borders(Borders::ALL)
            .border_style(files_border_style)
            .border_type(if !sidebar_focused {
                ratatui::widgets::BorderType::Double
            } else {
                ratatui::widgets::BorderType::Plain
            })
            .title(matches_title);

        let list_widget = List::new(items_list)
            .block(files_block)
            .style(Style::default().bg(theme.background));
        frame.render_stateful_widget(list_widget, list_area, list_state);

        // --- Render Footer ---
        let total_dirs = first_res.and_then(|r| r.metadata.get("total_dirs").map(|s| s.as_str())).unwrap_or("0");
        let total_files = first_res.and_then(|r| r.metadata.get("total_files").map(|s| s.as_str())).unwrap_or("0");

        let selected_desc = if !results.is_empty() && selected_idx < results.len() {
            let sel = &results[selected_idx];
            let name = sel.metadata.get("name").map(|s| s.as_str()).unwrap_or("");
            let size = sel.metadata.get("size").map(|s| s.as_str()).unwrap_or("");
            if name == ".." {
                if is_zh { "返回上一级".to_string() } else { "Go Up".to_string() }
            } else {
                format!("{name} ({size})")
            }
        } else {
            if is_zh { "无选中".to_string() } else { "No selection".to_string() }
        };

        let stats_str = if is_zh {
            format!(" 📁 {total_dirs} 个文件夹, 📄 {total_files} 个文件 │ 当前选中: {selected_desc} ")
        } else {
            format!(" 📁 {total_dirs} folders, 📄 {total_files} files │ Selected: {selected_desc} ")
        };

        let footer_text = if let Some(msg) = status_msg {
            Span::styled(msg, Style::default().fg(theme.success).add_modifier(Modifier::BOLD))
        } else {
            Span::styled(stats_str, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
        };

        let cheatsheet_text = if is_zh {
            " ⌨️ F2:关闭 | ←/→:切换焦点 | Alt-N:建文件夹 | Alt-F:建文件 | Alt-R:重命名 | Delete:删除 | F4:打开终端 "
        } else {
            " ⌨️ F2:Close | ←/→:Focus | Alt-N:Dir | Alt-F:File | Alt-R:Rename | Del:Delete | F4:Terminal "
        };

        let footer_p = Paragraph::new(Line::from(vec![
            footer_text,
            Span::raw(" | "),
            Span::styled(cheatsheet_text, Style::default().fg(theme.border).add_modifier(Modifier::DIM)),
        ]))
        .style(Style::default().bg(theme.background));
        frame.render_widget(footer_p, footer_area);

        return;
    }

    let (list_area, breadcrumb_p, header_p) = if is_file_manager {
        let list_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Breadcrumbs path
                Constraint::Length(1), // Columns header
                Constraint::Min(1),    // List itself
            ])
            .split(left_area);
        
        let current_dir_str = results.first()
            .and_then(|r| r.metadata.get("current_dir").cloned())
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .unwrap_or_else(|_| dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/home/fd")))
                    .to_string_lossy()
                    .to_string()
            });
        let breadcrumb_text = format_breadcrumbs(&current_dir_str, is_zh);
        let breadcrumb_p = Paragraph::new(Line::from(Span::styled(
            breadcrumb_text,
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)
        )));

        let avail_w = (left_area.width as usize).saturating_sub(6);
        let mod_w = 16;
        let size_w = 10;
        let name_w = avail_w.saturating_sub(mod_w + size_w + 2);
        
        let h_name = format!("{:<width$}", if is_zh { "名称" } else { "Name" }, width = name_w);
        let h_size = format!("{:>width$}", if is_zh { "大小" } else { "Size" }, width = size_w);
        let h_mod = format!("{:>width$}", if is_zh { "修改时间" } else { "Modified" }, width = mod_w);
        
        let header_text = format!("  {h_name}  {h_size}  {h_mod}");
        let header_p = Paragraph::new(Line::from(Span::styled(
            header_text,
            Style::default().fg(theme.border).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        )));
        
        (list_chunks[2], Some(breadcrumb_p), Some(header_p))
    } else {
        (left_area, None, None)
    };

    if let Some(bp) = breadcrumb_p {
        frame.render_widget(bp, left_area); // Renders inside its own split chunk
    }
    // Wait, let's render them in the specific splits
    if is_file_manager {
        let list_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Breadcrumbs path
                Constraint::Length(1), // Columns header
                Constraint::Min(1),    // List itself
            ])
            .split(left_area);
        
        if let Some(ref bp) = results.first()
            .and_then(|r| r.metadata.get("current_dir").cloned()) {
            let breadcrumb_text = format_breadcrumbs(bp, is_zh);
            let bp_widget = Paragraph::new(Line::from(Span::styled(
                breadcrumb_text,
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)
            )));
            frame.render_widget(bp_widget, list_chunks[0]);
        } else {
            let current_dir_str = std::env::current_dir()
                .unwrap_or_else(|_| dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/home/fd")))
                .to_string_lossy()
                .to_string();
            let breadcrumb_text = format_breadcrumbs(&current_dir_str, is_zh);
            let bp_widget = Paragraph::new(Line::from(Span::styled(
                breadcrumb_text,
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)
            )));
            frame.render_widget(bp_widget, list_chunks[0]);
        }

        if let Some(hp) = header_p {
            frame.render_widget(hp, list_chunks[1]);
        }
    }

    // Render Search Results List
    let items_list: Vec<ListItem> = results
        .iter()
        .enumerate()
        .map(|(idx, res)| {
            let is_selected = idx == selected_idx;
            let style = if is_selected {
                Style::default()
                    .bg(theme.selection)
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.foreground)
            };

            let mut lines = vec![];
            let prefix = if is_selected { "▶ " } else { "  " };

            if res.plugin_id == "file_manager" {
                let name = res.metadata.get("name").cloned().unwrap_or_else(|| res.title.clone());
                let icon = res.metadata.get("icon").map(|s| s.as_str()).unwrap_or("📄");
                let size = res.metadata.get("size").cloned().unwrap_or_default();
                let modified = res.metadata.get("modified").cloned().unwrap_or_default();

                let avail_w = (left_area.width as usize).saturating_sub(6);
                let mod_w = 16;
                let size_w = 10;
                let name_w = avail_w.saturating_sub(mod_w + size_w + 2);

                let mut name_part = format!("{icon} {name}");
                if name_part.len() > name_w {
                    name_part.truncate(name_w.saturating_sub(3));
                    name_part.push_str("...");
                }
                
                let name_padded = format!("{:<width$}", name_part, width = name_w);
                let size_padded = format!("{:>width$}", size, width = size_w);
                let mod_padded = format!("{:>width$}", modified, width = mod_w);

                let display_str = format!("{prefix}{name_padded}  {size_padded}  {mod_padded}");
                lines.push(Line::from(Span::styled(
                    display_str,
                    Style::default().fg(if is_selected { theme.accent } else { theme.foreground }).add_modifier(if is_selected { Modifier::BOLD } else { Modifier::empty() }),
                )));
            } else {
                // Build Title span
                let title_span = Span::styled(
                    format!("{prefix}{}", res.title),
                    Style::default().fg(if is_selected { theme.accent } else { theme.foreground }),
                );
                
                // Subtitle or plugin indicator
                let mut line_spans = vec![title_span];
                if let Some(ref sub) = res.subtitle {
                    let sub_truncated = if sub.len() > 45 {
                        format!(" | {}...", &sub[..42])
                    } else {
                        format!(" | {sub}")
                    };
                    line_spans.push(Span::styled(
                        sub_truncated,
                        Style::default().fg(theme.border).add_modifier(Modifier::DIM),
                    ));
                }

                lines.push(Line::from(line_spans));
            }

            ListItem::new(lines).style(style)
        })
        .collect();

    let matches_title = if is_zh {
        format!(" 匹配结果 (找到 {} 个) ", results.len())
    } else {
        format!(" Matches (Found {}) ", results.len())
    };

    let results_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(
            matches_title,
            Style::default().fg(theme.border),
        ));

    let list_widget = List::new(items_list)
        .block(results_block)
        .style(Style::default().bg(theme.background));

    // Stateful render to enable list auto-scrolling
    frame.render_stateful_widget(list_widget, list_area, list_state);

    // Render Preview Box (if available)
    if let Some((r_area, text)) = right_area {
        let title_str = if is_zh {
            if preview_scroll > 0 {
                format!(" 详情预览 [第 {} 行] ", preview_scroll)
            } else {
                " 详情预览 ".to_string()
            }
        } else {
            if preview_scroll > 0 {
                format!(" Detail Preview [Row {}] ", preview_scroll)
            } else {
                " Detail Preview ".to_string()
            }
        };

        let preview_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title(Span::styled(title_str, Style::default().fg(theme.border)));

        let mut lines = Vec::new();
        let mut in_code_block = false;

        for line in text.lines() {
            if line.starts_with("[IMAGE: ") && line.ends_with(']') {
                let path_str = &line[8..line.len() - 1];
                lines.extend(draw_image_in_preview(path_str, theme, 36, 12));
                continue;
            }
            if line.trim().starts_with("```") {
                in_code_block = !in_code_block;
                continue;
            }
            if in_code_block {
                lines.push(Line::from(Span::styled(
                    line.to_string(),
                    Style::default().fg(theme.success),
                )));
            } else {
                lines.push(parse_markdown_line(line, theme));
            }
        }

        let preview_p = Paragraph::new(lines)
            .block(preview_block)
            .scroll((preview_scroll, 0))
            .style(Style::default().bg(theme.background));

        frame.render_widget(preview_p, r_area);
    }

    // 3. Render Footer (status line & keybindings helper)
    let footer_text = if let Some(msg) = status_msg {
        Span::styled(msg, Style::default().fg(theme.success).add_modifier(Modifier::BOLD))
    } else if is_file_manager {
        let first_res = results.iter().find(|r| r.plugin_id == "file_manager");
        let total_dirs = first_res.and_then(|r| r.metadata.get("total_dirs").map(|s| s.as_str())).unwrap_or("0");
        let total_files = first_res.and_then(|r| r.metadata.get("total_files").map(|s| s.as_str())).unwrap_or("0");
        
        let selected_desc = if !results.is_empty() && selected_idx < results.len() {
            let sel = &results[selected_idx];
            let name = sel.metadata.get("name").map(|s| s.as_str()).unwrap_or("");
            let size = sel.metadata.get("size").map(|s| s.as_str()).unwrap_or("");
            if name == ".." {
                if is_zh { "返回上一级".to_string() } else { "Go Up".to_string() }
            } else {
                format!("{name} ({size})")
            }
        } else {
            if is_zh { "无选中".to_string() } else { "No selection".to_string() }
        };

        let stats_str = if is_zh {
            format!(" 📁 {total_dirs} 个文件夹, 📄 {total_files} 个文件 │ 当前选中: {selected_desc} ")
        } else {
            format!(" 📁 {total_dirs} folders, 📄 {total_files} files │ Selected: {selected_desc} ")
        };

        Span::styled(stats_str, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
    } else if preview_content.is_some() {
        Span::styled(
            if is_zh {
                " Enter 启动 │ Tab 切换模式 │ Shift-↑/↓ 滚动详情 │ Esc 退出 │ F1 设置 "
            } else {
                " Enter Launch │ Tab Mode │ Shift-↑/↓ Scroll Detail │ Esc Exit │ F1 Settings "
            },
            Style::default().fg(theme.border),
        )
    } else {
        Span::styled(
            if is_zh {
                " Enter 启动 │ Tab 切换模式 │ Esc 退出 │ F1 设置 "
            } else {
                " Enter Launch │ Tab Mode │ Esc Exit │ F1 Settings "
            },
            Style::default().fg(theme.border),
        )
    };

    let footer_p = Paragraph::new(Line::from(vec![
        footer_text,
        Span::raw(" | "),
        Span::styled(
            if is_zh {
                format!("Rune: {} 个激活插件", total_plugins_count)
            } else {
                format!("Rune: {} active plugins", total_plugins_count)
            },
            Style::default().fg(theme.border).add_modifier(Modifier::DIM),
        ),
    ]))
    .style(Style::default().bg(theme.background));

    frame.render_widget(footer_p, chunks[2]);
}
