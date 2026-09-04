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

fn fixed_centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(r.height.saturating_sub(height) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(r.width.saturating_sub(width) / 2),
            Constraint::Length(width),
            Constraint::Min(0),
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

    // Left Category List (Expanded to 11 categories)
    let categories = if is_zh {
        vec![
            "1. 主题选择 (TUI Theme)",
            "2. 语言切换 (UI Language)",
            "3. 文本编辑器 (Text Editor)",
            "4. 终端 Shell (Terminal Shell)",
            "5. 字体选择 (TUI Font)",
            "6. 插件管理 (Active Plugins)",
            "7. 文件搜索设置 (File Search)",
            "8. 文件管理器设置 (File Manager)",
            "9. AI 提供商 (AI Provider)",
            "10. AI 参数配置 (AI Credentials)",
            "11. 关于项目 (About Rune)",
        ]
    } else {
        vec![
            "1. TUI Theme",
            "2. UI Language",
            "3. Text Editor",
            "4. Terminal Shell",
            "5. TUI Font",
            "6. Active Plugins",
            "7. File Search Settings",
            "8. File Manager Settings",
            "9. AI Provider",
            "10. AI Credentials",
            "11. About Rune",
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
            let transparent_str = if config.theme.transparent {
                if is_zh { "透明背景 (开启)" } else { "Transparent Background (ON)" }
            } else {
                if is_zh { "透明背景 (关闭)" } else { "Transparent Background (OFF)" }
            };
            options = vec![
                "catppuccin".to_string(),
                "tokyo_night".to_string(),
                "nord".to_string(),
                "gruvbox".to_string(),
                "everforest".to_string(),
                "transparent".to_string(),
                transparent_str.to_string(),
            ];
            active_val = config.theme.active.clone();
            desc = if is_zh {
                "选择主界面的着色主题，或开关透明背景支持。按 Enter 键可实时切换预览并保存。配合终端背景透明/模糊效果更佳。".to_string()
            } else {
                "Select UI color theme or toggle transparent background mode. Press Enter to live preview and switch.".to_string()
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
                "bash".to_string(),
                "zsh".to_string(),
                "sh".to_string(),
                format!("custom ({})", config.general.shell),
            ];
            active_val = config.general.shell.clone();
            desc = if is_zh {
                "设置默认终端 Shell。选择最后一项按 Enter 可手动输入自定义 Shell。".to_string()
            } else {
                "Configure default terminal shell. Select the last option and press Enter to type custom shell.".to_string()
            };
        }
        4 => {
            options = _scanned_fonts.to_vec();
            active_val = config.general.font.clone();
            desc = if is_zh {
                "选择主界面所用字体。字体更改适用于支持字体切换的终端环境。".to_string()
            } else {
                "Select monospace UI font. Applies only if terminal shell supports font rendering.".to_string()
            };
        }
        5 => {
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
        6 => {
            options = vec![
                format!("Max Depth: {}", config.plugins.files_max_depth),
                format!("Paths: {}", config.plugins.files_paths.join(", ")),
                format!("Ignore: {}", config.plugins.files_ignore.join(", ")),
            ];
            desc = if is_zh {
                "配置本地文件搜索的相关参数。包括遍历最大目录级数，以及扫描根目录和忽略黑名单目录，支持以逗号分隔输入多个路径。".to_string()
            } else {
                "Configure file scanner parameters. Includes max search depth, scan paths, and ignored directories (supports comma-separated list).".to_string()
            };
        }
        7 => {
            let show_hidden_str = if config.plugins.file_manager_show_hidden {
                if is_zh { "显示隐藏文件 (开启)" } else { "Show Hidden Files (ON)" }
            } else {
                if is_zh { "显示隐藏文件 (关闭)" } else { "Show Hidden Files (OFF)" }
            };
            options = vec![
                show_hidden_str.to_string(),
                format!("Start Dir: {}", config.plugins.file_manager_start_dir),
            ];
            desc = if is_zh {
                "配置文件管理器的行为参数。第一个选项可开启或隐藏以点(.)开头的隐藏文件，第二个选项可自定义文件管理器的起始工作目录。".to_string()
            } else {
                "Configure file manager settings. Toggles visibility of files starting with dot (.), and specifies initial directory for the file manager.".to_string()
            };
        }
        8 => {
            options = vec!["openai".to_string(), "gemini".to_string(), "ollama".to_string()];
            active_val = config.plugins.ai_provider.clone();
            desc = if is_zh {
                "选择用于 AI 助手插件的底层服务模型提供商。可在 config.toml 中配置对应的 API Key。".to_string()
            } else {
                "Select AI model provider used by the AI chatbot plugin. Make sure to configure the API key in config.toml.".to_string()
            };
        }
        9 => {
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
        10 => {
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
            
            let is_active = if settings_selected_category == 5 {
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
            } else if settings_selected_category == 6 || settings_selected_category == 10 {
                false
            } else if settings_selected_category == 2 {
                if idx < 4 {
                    opt == &active_val
                } else {
                    !["nano", "vim", "nvim", "hx"].contains(&active_val.as_str())
                }
            } else if settings_selected_category == 3 {
                if idx < 3 {
                    opt == &active_val
                } else {
                    !["bash", "zsh", "sh"].contains(&active_val.as_str())
                }
            } else if settings_selected_category == 0 {
                if idx < 6 {
                    opt == &active_val
                } else {
                    config.theme.transparent
                }
            } else if settings_selected_category == 7 {
                if idx == 0 {
                    config.plugins.file_manager_show_hidden
                } else {
                    false
                }
            } else {
                opt == &active_val
            };
            
            let style = if settings_selected_category == 10 {
                Style::default().fg(theme.foreground)
            } else if is_focused {
                Style::default().bg(theme.selection).fg(theme.accent).add_modifier(Modifier::BOLD)
            } else if is_hovered {
                Style::default().bg(theme.selection).fg(theme.foreground)
            } else {
                Style::default().fg(theme.foreground)
            };
            
            let prefix = if settings_selected_category == 10 {
                ""
            } else if is_hovered {
                "▶ "
            } else {
                "  "
            };
            let checked = if settings_selected_category == 6 || settings_selected_category == 9 || settings_selected_category == 10 {
                ""
            } else if settings_selected_category == 7 && idx == 1 {
                "   "
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
            3 => if is_zh { "编辑 Shell 命令: " } else { "Edit Shell Command: " },
            6 => match settings_selected_option {
                0 => if is_zh { "编辑文件搜索最大深度: " } else { "Edit Max Depth: " },
                1 => if is_zh { "编辑搜索路径 (逗号分隔): " } else { "Edit Search Paths: " },
                2 => if is_zh { "编辑忽略目录 (逗号分隔): " } else { "Edit Ignore Paths: " },
                _ => "",
            },
            7 => match settings_selected_option {
                1 => if is_zh { "编辑文件管理器起始工作目录: " } else { "Edit Start Directory: " },
                _ => "",
            },
            9 => match settings_selected_option {
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
    main_focus_pane: usize,
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

        let fav_area = sidebar_area;

        let first_res = results.iter().find(|r| r.plugin_id == "file_manager");
        let sidebar_focused = first_res
            .and_then(|r| r.metadata.get("sidebar_focused").map(|s| s == "true"))
            .unwrap_or(false);
        let sidebar_selected_idx = first_res
            .and_then(|r| r.metadata.get("sidebar_selected_idx").and_then(|s| s.parse::<usize>().ok()))
            .unwrap_or(0);

        // --- Render Favorites & Custom Groups ---
        let home_c = first_res.and_then(|r| r.metadata.get("fav_home_count").map(|s| s.as_str())).unwrap_or("0");
        let desktop_c = first_res.and_then(|r| r.metadata.get("fav_desktop_count").map(|s| s.as_str())).unwrap_or("0");
        let docs_c = first_res.and_then(|r| r.metadata.get("fav_docs_count").map(|s| s.as_str())).unwrap_or("0");
        let downloads_c = first_res.and_then(|r| r.metadata.get("fav_downloads_count").map(|s| s.as_str())).unwrap_or("0");
        let music_c = first_res.and_then(|r| r.metadata.get("fav_music_count").map(|s| s.as_str())).unwrap_or("0");
        let pics_c = first_res.and_then(|r| r.metadata.get("fav_pics_count").map(|s| s.as_str())).unwrap_or("0");
        let videos_c = first_res.and_then(|r| r.metadata.get("fav_videos_count").map(|s| s.as_str())).unwrap_or("0");
        let trash_c = first_res.and_then(|r| r.metadata.get("fav_trash_count").map(|s| s.as_str())).unwrap_or("0");

        let fav_items = if is_zh {
            vec![
                (format!("🏠 主文件夹 ({})", home_c), 0),
                (format!("🖥️ 桌面 ({})", desktop_c), 1),
                (format!("📄 文档 ({})", docs_c), 2),
                (format!("📥 下载 ({})", downloads_c), 3),
                (format!("🎵 音乐 ({})", music_c), 4),
                (format!("📷 图片 ({})", pics_c), 5),
                (format!("🎥 视频 ({})", videos_c), 6),
                (format!("🗑️ 回收站 ({})", trash_c), 7),
            ]
        } else {
            vec![
                (format!("🏠 Home ({})", home_c), 0),
                (format!("🖥️ Desktop ({})", desktop_c), 1),
                (format!("📄 Documents ({})", docs_c), 2),
                (format!("📥 Downloads ({})", downloads_c), 3),
                (format!("🎵 Music ({})", music_c), 4),
                (format!("📷 Pictures ({})", pics_c), 5),
                (format!("🎥 Videos ({})", videos_c), 6),
                (format!("🗑️ Trash ({})", trash_c), 7),
            ]
        };

        let mut visual_items = Vec::new();

        // 1. Places Header
        visual_items.push((if is_zh { "常用位置".to_string() } else { "Places".to_string() }, false, None));
        for (label, idx) in fav_items {
            visual_items.push((label, true, Some(idx)));
        }

        // 2. Recently Used Header
        visual_items.push((if is_zh { "最近使用".to_string() } else { "Recently Used".to_string() }, false, None));
        visual_items.push((if is_zh { "⏱️ 最近文件".to_string() } else { "⏱️ Recent Files".to_string() }, true, Some(8)));
        visual_items.push((if is_zh { "📂 最近位置".to_string() } else { "📂 Recent Locations".to_string() }, true, Some(9)));

        // 3. Storage Devices Header
        visual_items.push((if is_zh { "存储设备".to_string() } else { "Storage Devices".to_string() }, false, None));
        let drives_count = first_res.and_then(|r| r.metadata.get("fav_drives_count").and_then(|s| s.parse::<usize>().ok())).unwrap_or(0);
        for d in 0..drives_count {
            let name = first_res.and_then(|r| r.metadata.get(&format!("fav_drive_{}_name", d))).cloned().unwrap_or_default();
            let kind = first_res.and_then(|r| r.metadata.get(&format!("fav_drive_{}_kind", d))).map(|s| s.as_str()).unwrap_or("drive_root");
            let icon = if kind == "drive_root" { "💽 " } else { "💾 " };
            visual_items.push((format!("{}{}", icon, name), true, Some(10 + d)));
        }

        let fav_title = if is_zh { " 导航栏 " } else { " Navigation " };
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

        let list_items: Vec<ListItem> = visual_items
            .iter()
            .map(|(label, is_selectable, opt_idx)| {
                if !is_selectable {
                    ListItem::new(Line::from(vec![
                        Span::styled(format!(" {}", label), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                    ]))
                } else {
                    let idx = opt_idx.unwrap();
                    let is_selected = idx == sidebar_selected_idx;
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
                }
            })
            .collect();

        let fav_list = List::new(list_items)
            .block(fav_block)
            .style(Style::default().bg(theme.background));
        frame.render_widget(fav_list, fav_area);

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
            " ⌨️ F2:关闭 | m:操作菜单 | Alt-H:显示隐藏 | ←/→:焦点 | Alt-N:建文件夹 | Alt-F:建文件 | Del:删除 | F4:终端 "
        } else {
            " ⌨️ F2:Close | m:Menu | Alt-H:Hidden | ←/→:Focus | Alt-N:Dir | Alt-F:File | Del:Delete | F4:Terminal "
        };

        let footer_p = Paragraph::new(Line::from(vec![
            footer_text,
            Span::raw(" | "),
            Span::styled(cheatsheet_text, Style::default().fg(theme.border).add_modifier(Modifier::DIM)),
        ]))
        .style(Style::default().bg(theme.background));
        frame.render_widget(footer_p, footer_area);

        // Draw Context Menu Popup if open
        let context_menu_open = first_res
            .and_then(|r| r.metadata.get("context_menu_open").map(|s| s == "true"))
            .unwrap_or(false);
        if context_menu_open {
            let context_menu_selected_idx = first_res
                .and_then(|r| r.metadata.get("context_menu_selected_idx").and_then(|s| s.parse::<usize>().ok()))
                .unwrap_or(0);

            let menu_options = vec![
                if is_zh { "▶ 打开 (Enter)" } else { "▶ Open (Enter)" },
                if is_zh { "📋 复制 (Copy)" } else { "📋 Copy" },
                if is_zh { "✂️ 剪切 (Cut)" } else { "✂️ Cut" },
                if is_zh { "📥 粘贴 (Paste)" } else { "📥 Paste" },
                if is_zh { "✏️ 重命名 (Rename)" } else { "✏️ Rename" },
                if is_zh { "🗑️ 删除 (Delete)" } else { "🗑️ Delete" },
                if is_zh { "📄 新建文件 (New File)" } else { "📄 New File" },
                if is_zh { "📁 新建文件夹 (New Dir)" } else { "📁 New Folder" },
                if is_zh { "ℹ️ 属性 (Properties)" } else { "ℹ️ Properties" },
                if is_zh { "❌ 取消 (Cancel)" } else { "❌ Cancel" },
            ];

            let menu_title = if is_zh { " 操作菜单 " } else { " Context Menu " };
            let menu_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
                .title(menu_title);

            let selected_idx = list_state.selected().unwrap_or(0);
            let selected_item = if !results.is_empty() && selected_idx < results.len() {
                let item = &results[selected_idx];
                if item.id == "dummy_metadata_carrier" {
                    None
                } else {
                    Some(item)
                }
            } else {
                None
            };

            let has_selection = selected_item.is_some();
            let has_valid_selection = if let Some(item) = selected_item {
                item.id != ".."
            } else {
                false
            };

            let opt_items: Vec<ListItem> = menu_options
                .iter()
                .enumerate()
                .map(|(idx, opt)| {
                    let is_hovered = idx == context_menu_selected_idx;
                    let is_disabled = match idx {
                        0 => !has_selection,
                        1 | 2 | 4 | 5 | 8 => !has_valid_selection,
                        _ => false,
                    };
                    
                    let style = if is_disabled {
                        if is_hovered {
                            Style::default().bg(theme.selection).fg(theme.border).add_modifier(Modifier::DIM)
                        } else {
                            Style::default().fg(theme.border).add_modifier(Modifier::DIM)
                        }
                    } else if is_hovered {
                        Style::default().bg(theme.selection).fg(theme.accent).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.foreground)
                    };
                    
                    let opt_text = if is_disabled {
                        format!("{} (N/A)", opt)
                    } else {
                        opt.to_string()
                    };
                    
                    ListItem::new(Line::from(Span::styled(opt_text, style)))
                })
                .collect();
            let opt_list = List::new(opt_items).block(menu_block).style(Style::default().bg(theme.background));
            
            let popup_area = fixed_centered_rect(32, 12, area);
            frame.render_widget(Clear, popup_area); // clear underneath
            frame.render_widget(opt_list, popup_area);
        }

        // Draw Input Dialog Popup if open
        let input_dialog_open = first_res
            .and_then(|r| r.metadata.get("input_dialog_open").map(|s| s == "true"))
            .unwrap_or(false);
        if input_dialog_open {
            let input_title = first_res
                .and_then(|r| r.metadata.get("input_dialog_title").cloned())
                .unwrap_or_else(|| "Input".to_string());
            let input_buf = first_res
                .and_then(|r| r.metadata.get("input_dialog_buffer").cloned())
                .unwrap_or_default();

            let popup_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
                .title(format!(" {} ", input_title));

            let input_p = Paragraph::new(Line::from(vec![
                Span::raw(" > "),
                Span::styled(input_buf.clone(), Style::default().fg(theme.foreground)),
            ]))
            .block(popup_block)
            .style(Style::default().bg(theme.background));

            let popup_area = fixed_centered_rect(50, 3, area);
            frame.render_widget(Clear, popup_area); // clear underneath
            frame.render_widget(input_p, popup_area);

            // Set terminal cursor dynamically at end of input buffer
            let cursor_x = popup_area.x + 3 + input_buf.chars().count() as u16;
            let cursor_y = popup_area.y + 1;
            frame.set_cursor(cursor_x, cursor_y);
        }

        // Draw Properties Dialog if open
        let properties_dialog_open = first_res
            .and_then(|r| r.metadata.get("properties_dialog_open").map(|s| s == "true"))
            .unwrap_or(false);
        if properties_dialog_open {
            let item_id = first_res
                .and_then(|r| r.metadata.get("properties_dialog_item_id").cloned())
                .unwrap_or_default();
            let path = std::path::Path::new(&item_id);
            
            let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let is_dir = path.is_dir();
            let type_desc = crate::plugins::file_manager::get_file_type_desc(path, is_dir);
            
            let size_str = if is_dir {
                results.iter()
                    .find(|r| r.id == item_id)
                    .and_then(|r| r.metadata.get("size").cloned())
                    .unwrap_or_else(|| "unknown".to_string())
            } else {
                std::fs::metadata(path)
                    .map(|m| {
                        let size = m.len();
                        if size < 1024 {
                            format!("{} B", size)
                        } else if size < 1024 * 1024 {
                            format!("{:.1} KB", size as f64 / 1024.0)
                        } else {
                            format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
                        }
                    })
                    .unwrap_or_else(|_| "unknown".to_string())
            };

            let permissions = crate::plugins::file_manager::get_permissions_str(path);
            let owner = crate::plugins::file_manager::get_owner_str(path);

            let properties_title = if is_zh { " 属性信息 " } else { " Properties " };
            let popup_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
                .title(properties_title);

            let info_lines = vec![
                Line::from(vec![
                    Span::styled(format!("  {:<12}", if is_zh { "名称:" } else { "Name:" }), Style::default().fg(theme.border)),
                    Span::styled(name, Style::default().fg(theme.foreground).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::styled(format!("  {:<12}", if is_zh { "类型:" } else { "Type:" }), Style::default().fg(theme.border)),
                    Span::styled(type_desc, Style::default().fg(theme.foreground)),
                ]),
                Line::from(vec![
                    Span::styled(format!("  {:<12}", if is_zh { "大小:" } else { "Size:" }), Style::default().fg(theme.border)),
                    Span::styled(size_str, Style::default().fg(theme.foreground)),
                ]),
                Line::from(vec![
                    Span::styled(format!("  {:<12}", if is_zh { "路径:" } else { "Path:" }), Style::default().fg(theme.border)),
                    Span::styled(item_id, Style::default().fg(theme.foreground).add_modifier(Modifier::DIM)),
                ]),
                Line::from(vec![
                    Span::styled(format!("  {:<12}", if is_zh { "权限:" } else { "Perms:" }), Style::default().fg(theme.border)),
                    Span::styled(permissions, Style::default().fg(theme.success)),
                ]),
                Line::from(vec![
                    Span::styled(format!("  {:<12}", if is_zh { "所有者:" } else { "Owner:" }), Style::default().fg(theme.border)),
                    Span::styled(owner, Style::default().fg(theme.foreground)),
                ]),
                Line::from(vec![]),
                Line::from(vec![
                    Span::styled(if is_zh { "  按任意键关闭..." } else { "  Press any key to close..." }, Style::default().fg(theme.border).add_modifier(Modifier::DIM)),
                ]),
            ];

            let info_p = Paragraph::new(info_lines)
                .block(popup_block)
                .style(Style::default().bg(theme.background));

            let popup_area = fixed_centered_rect(60, 10, area);
            frame.render_widget(Clear, popup_area); // clear underneath
            frame.render_widget(info_p, popup_area);
        }

        // Draw Delete Confirmation Dialog if open
        let delete_confirm_open = first_res
            .and_then(|r| r.metadata.get("delete_confirm_open").map(|s| s == "true"))
            .unwrap_or(false);
        if delete_confirm_open {
            let item_id = first_res
                .and_then(|r| r.metadata.get("delete_confirm_item_id").cloned())
                .unwrap_or_default();
            let delete_confirm_selected_idx = first_res
                .and_then(|r| r.metadata.get("delete_confirm_selected_idx").and_then(|s| s.parse::<usize>().ok()))
                .unwrap_or(0);

            let filename = std::path::Path::new(&item_id)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            let confirm_title = if is_zh { " 确认删除 " } else { " Confirm Delete " };
            let popup_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.error).add_modifier(Modifier::BOLD))
                .title(confirm_title);

            let question = if is_zh {
                format!("您确定要永久删除 '{}' 吗？", filename)
            } else {
                format!("Are you sure you want to permanently delete '{}'?", filename)
            };

            let btn_no_style = if delete_confirm_selected_idx == 0 {
                Style::default().bg(theme.selection).fg(theme.accent).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.foreground)
            };

            let btn_yes_style = if delete_confirm_selected_idx == 1 {
                Style::default().bg(theme.selection).fg(theme.error).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.foreground)
            };

            let btn_no = if is_zh { "[ 否 (N) ]" } else { "[ No (N) ]" };
            let btn_yes = if is_zh { "[ 是 (Y) ]" } else { "[ Yes (Y) ]" };

            let confirm_lines = vec![
                Line::from(vec![]),
                Line::from(vec![
                    Span::styled(format!("  {}", question), Style::default().fg(theme.foreground).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![]),
                Line::from(vec![
                    Span::raw("      "),
                    Span::styled(btn_no, btn_no_style),
                    Span::raw("          "),
                    Span::styled(btn_yes, btn_yes_style),
                ]),
            ];

            let confirm_p = Paragraph::new(confirm_lines)
                .block(popup_block)
                .style(Style::default().bg(theme.background));

            let popup_area = fixed_centered_rect(55, 6, area);
            frame.render_widget(Clear, popup_area); // clear underneath
            frame.render_widget(confirm_p, popup_area);
        }

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
    let is_search_focused = main_focus_pane == 0;
    let search_block = Block::default()
        .title(Span::styled(search_title, Style::default().fg(if is_search_focused { theme.accent } else { theme.border })))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if is_search_focused { theme.accent } else { theme.border }))
        .border_type(if is_search_focused {
            ratatui::widgets::BorderType::Double
        } else {
            ratatui::widgets::BorderType::Plain
        });

    // Show text cursor at end of query
    let cursor_span = Span::raw(query);
    let search_p = Paragraph::new(Line::from(vec![
        Span::styled("🔍 ", Style::default().fg(theme.accent)),
        cursor_span,
    ]))
    .block(search_block)
    .style(Style::default().bg(theme.background));

    frame.render_widget(search_p, chunks[0]);

    if is_search_focused {
        // Place terminal cursor in the search input box (offset: 1 border + 3 for "🔍 " emoji)
        let cursor_x = (chunks[0].x + 4 + query.chars().count() as u16)
            .min(chunks[0].x + chunks[0].width.saturating_sub(2));
        let cursor_y = chunks[0].y + 1;
        frame.set_cursor(cursor_x, cursor_y);
    }

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
    let focus_pane = 1;

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

        let fav_area = sidebar_area;

        let first_res = results.iter().find(|r| r.plugin_id == "file_manager");
        let sidebar_focused = first_res
            .and_then(|r| r.metadata.get("sidebar_focused").map(|s| s == "true"))
            .unwrap_or(false);
        let sidebar_selected_idx = first_res
            .and_then(|r| r.metadata.get("sidebar_selected_idx").and_then(|s| s.parse::<usize>().ok()))
            .unwrap_or(0);

        // --- Render Favorites & Custom Groups ---
        let home_c = first_res.and_then(|r| r.metadata.get("fav_home_count").map(|s| s.as_str())).unwrap_or("0");
        let desktop_c = first_res.and_then(|r| r.metadata.get("fav_desktop_count").map(|s| s.as_str())).unwrap_or("0");
        let docs_c = first_res.and_then(|r| r.metadata.get("fav_docs_count").map(|s| s.as_str())).unwrap_or("0");
        let downloads_c = first_res.and_then(|r| r.metadata.get("fav_downloads_count").map(|s| s.as_str())).unwrap_or("0");
        let music_c = first_res.and_then(|r| r.metadata.get("fav_music_count").map(|s| s.as_str())).unwrap_or("0");
        let pics_c = first_res.and_then(|r| r.metadata.get("fav_pics_count").map(|s| s.as_str())).unwrap_or("0");
        let videos_c = first_res.and_then(|r| r.metadata.get("fav_videos_count").map(|s| s.as_str())).unwrap_or("0");
        let trash_c = first_res.and_then(|r| r.metadata.get("fav_trash_count").map(|s| s.as_str())).unwrap_or("0");

        let fav_items = if is_zh {
            vec![
                (format!("🏠 主文件夹 ({})", home_c), 0),
                (format!("🖥️ 桌面 ({})", desktop_c), 1),
                (format!("📄 文档 ({})", docs_c), 2),
                (format!("📥 下载 ({})", downloads_c), 3),
                (format!("🎵 音乐 ({})", music_c), 4),
                (format!("📷 图片 ({})", pics_c), 5),
                (format!("🎥 视频 ({})", videos_c), 6),
                (format!("🗑️ 回收站 ({})", trash_c), 7),
            ]
        } else {
            vec![
                (format!("🏠 Home ({})", home_c), 0),
                (format!("🖥️ Desktop ({})", desktop_c), 1),
                (format!("📄 Documents ({})", docs_c), 2),
                (format!("📥 Downloads ({})", downloads_c), 3),
                (format!("🎵 Music ({})", music_c), 4),
                (format!("📷 Pictures ({})", pics_c), 5),
                (format!("🎥 Videos ({})", videos_c), 6),
                (format!("🗑️ Trash ({})", trash_c), 7),
            ]
        };

        let mut visual_items = Vec::new();

        // 1. Places Header
        visual_items.push((if is_zh { "常用位置".to_string() } else { "Places".to_string() }, false, None));
        for (label, idx) in fav_items {
            visual_items.push((label, true, Some(idx)));
        }

        // 2. Recently Used Header
        visual_items.push((if is_zh { "最近使用".to_string() } else { "Recently Used".to_string() }, false, None));
        visual_items.push((if is_zh { "⏱️ 最近文件".to_string() } else { "⏱️ Recent Files".to_string() }, true, Some(8)));
        visual_items.push((if is_zh { "📂 最近位置".to_string() } else { "📂 Recent Locations".to_string() }, true, Some(9)));

        // 3. Storage Devices Header
        visual_items.push((if is_zh { "存储设备".to_string() } else { "Storage Devices".to_string() }, false, None));
        let drives_count = first_res.and_then(|r| r.metadata.get("fav_drives_count").and_then(|s| s.parse::<usize>().ok())).unwrap_or(0);
        for d in 0..drives_count {
            let name = first_res.and_then(|r| r.metadata.get(&format!("fav_drive_{}_name", d))).cloned().unwrap_or_default();
            let kind = first_res.and_then(|r| r.metadata.get(&format!("fav_drive_{}_kind", d))).map(|s| s.as_str()).unwrap_or("drive_root");
            let icon = if kind == "drive_root" { "💽 " } else { "💾 " };
            visual_items.push((format!("{}{}", icon, name), true, Some(10 + d)));
        }

        let fav_title = if is_zh { " 导航栏 " } else { " Navigation " };
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

        let list_items: Vec<ListItem> = visual_items
            .iter()
            .map(|(label, is_selectable, opt_idx)| {
                if !is_selectable {
                    ListItem::new(Line::from(vec![
                        Span::styled(format!(" {}", label), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                    ]))
                } else {
                    let idx = opt_idx.unwrap();
                    let is_selected = idx == sidebar_selected_idx;
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
                }
            })
            .collect();

        let fav_list = List::new(list_items)
            .block(fav_block)
            .style(Style::default().bg(theme.background));
        frame.render_widget(fav_list, fav_area);

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
        let files_focused = focus_pane == 1;
        let files_border_style = if files_focused {
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
            .border_type(if files_focused {
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
            " ⌨️ F2:关闭 | m:操作菜单 | Alt-H:显示隐藏 | ←/→:焦点 | Alt-N:建文件夹 | Alt-F:建文件 | Del:删除 | F4:终端 "
        } else {
            " ⌨️ F2:Close | m:Menu | Alt-H:Hidden | ←/→:Focus | Alt-N:Dir | Alt-F:File | Del:Delete | F4:Terminal "
        };

        let footer_p = Paragraph::new(Line::from(vec![
            footer_text,
            Span::raw(" | "),
            Span::styled(cheatsheet_text, Style::default().fg(theme.border).add_modifier(Modifier::DIM)),
        ]))
        .style(Style::default().bg(theme.background));
        frame.render_widget(footer_p, footer_area);

        // Draw Context Menu Popup if open
        let context_menu_open = first_res
            .and_then(|r| r.metadata.get("context_menu_open").map(|s| s == "true"))
            .unwrap_or(false);
        if context_menu_open {
            let context_menu_selected_idx = first_res
                .and_then(|r| r.metadata.get("context_menu_selected_idx").and_then(|s| s.parse::<usize>().ok()))
                .unwrap_or(0);

            let menu_options = vec![
                if is_zh { "▶ 打开 (Enter)" } else { "▶ Open (Enter)" },
                if is_zh { "📋 复制 (Copy)" } else { "📋 Copy" },
                if is_zh { "✂️ 剪切 (Cut)" } else { "✂️ Cut" },
                if is_zh { "📥 粘贴 (Paste)" } else { "📥 Paste" },
                if is_zh { "✏️ 重命名 (Rename)" } else { "✏️ Rename" },
                if is_zh { "🗑️ 删除 (Delete)" } else { "🗑️ Delete" },
                if is_zh { "📄 新建文件 (New File)" } else { "📄 New File" },
                if is_zh { "📁 新建文件夹 (New Dir)" } else { "📁 New Folder" },
                if is_zh { "ℹ️ 属性 (Properties)" } else { "ℹ️ Properties" },
                if is_zh { "❌ 取消 (Cancel)" } else { "❌ Cancel" },
            ];

            let menu_title = if is_zh { " 操作菜单 " } else { " Context Menu " };
            let menu_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
                .title(menu_title);

            let opt_items: Vec<ListItem> = menu_options
                .iter()
                .enumerate()
                .map(|(idx, opt)| {
                    let is_hovered = idx == context_menu_selected_idx;
                    let style = if is_hovered {
                        Style::default().bg(theme.selection).fg(theme.accent).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.foreground)
                    };
                    ListItem::new(Line::from(Span::styled(*opt, style)))
                })
                .collect();
            let opt_list = List::new(opt_items).block(menu_block).style(Style::default().bg(theme.background));
            
            let popup_area = fixed_centered_rect(32, 12, area);
            frame.render_widget(Clear, popup_area); // clear underneath
            frame.render_widget(opt_list, popup_area);
        }

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

    let is_results_focused = main_focus_pane == 1;
    let results_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if is_results_focused { theme.accent } else { theme.border }))
        .border_type(if is_results_focused {
            ratatui::widgets::BorderType::Double
        } else {
            ratatui::widgets::BorderType::Plain
        })
        .title(Span::styled(
            matches_title,
            Style::default().fg(if is_results_focused { theme.accent } else { theme.border }),
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
