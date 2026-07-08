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
    scanned_fonts: &[String],
    active_theme: &str,
    active_lang: &str,
    active_font: &str,
    active_shell: &str,
    active_editor: &str,
    status_msg: Option<&str>,
) {
    let is_zh = active_lang == "zh";
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

    // Left Category List
    let categories = if is_zh {
        vec![
            "1. 主题选择 (TUI Theme)",
            "2. 语言切换 (UI Language)",
            "3. 系统字体 (Monospace Font)",
            "4. 默认外壳 (Shell Executable)",
            "5. 文本编辑器 (Text Editor)",
        ]
    } else {
        vec![
            "1. TUI Theme",
            "2. UI Language",
            "3. Monospace Font",
            "4. Shell Executable",
            "5. Text Editor",
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
            active_val = active_theme.to_string();
            desc = if is_zh {
                "选择主界面的着色主题。支持 Tokyo Night, Catppuccin, Nord 等配色，按 Enter 键可 live 实时切换预览。".to_string()
            } else {
                "Select UI color theme. Supports Tokyo Night, Catppuccin, Nord, etc. Press Enter to live preview and switch.".to_string()
            };
        }
        1 => {
            options = vec!["zh".to_string(), "en".to_string()];
            active_val = active_lang.to_string();
            desc = if is_zh {
                "切换主界面的语言。支持中文 (zh) 和英文 (en)，按 Enter 确定切换。".to_string()
            } else {
                "Switch UI display language. Supports Chinese (zh) and English (en). Press Enter to apply.".to_string()
            };
        }
        2 => {
            options = scanned_fonts.to_vec();
            active_val = active_font.to_string();
            desc = if is_zh {
                "选择您系统已安装的等宽字体。注意：等宽字体需要您在自己的终端模拟器配置中加载生效，Rune 仅进行配置记录。".to_string()
            } else {
                "Select a system Monospace font. Note: Fonts are managed by your terminal emulator; Rune only records it in config.".to_string()
            };
        }
        3 => {
            options = vec!["bash".to_string(), "zsh".to_string(), "fish".to_string()];
            active_val = active_shell.to_string();
            desc = if is_zh {
                "设置用于启动后台/前台终端命令行任务的默认 Shell 运行程序。".to_string()
            } else {
                "Configure default Shell interpreter used to run foreground or background terminal commands.".to_string()
            };
        }
        4 => {
            options = vec!["nano".to_string(), "vim".to_string(), "nvim".to_string(), "hx".to_string()];
            active_val = active_editor.to_string();
            desc = if is_zh {
                "设置打开文本配置、外部修改时调用的默认终端编辑器。".to_string()
            } else {
                "Configure default terminal text editor to open configuration or script files.".to_string()
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
            let is_active = opt == &active_val;
            
            let style = if is_focused {
                Style::default().bg(theme.selection).fg(theme.accent).add_modifier(Modifier::BOLD)
            } else if is_hovered {
                Style::default().bg(theme.selection).fg(theme.foreground)
            } else {
                Style::default().fg(theme.foreground)
            };
            
            let prefix = if is_hovered { "▶ " } else { "  " };
            let checked = if is_active { " ✔ " } else { "   " };
            
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

    let status_text = if let Some(msg) = status_msg {
        Span::styled(msg, Style::default().fg(theme.success).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(
            if is_zh { "使用 ↑/↓ 移动选择，Enter 确认修改，Esc/F1 关闭" } else { "Use ↑/↓ to navigate, Enter to select, Esc/F1 to close" },
            Style::default().fg(theme.border),
        )
    };
    
    let status_p = Paragraph::new(Line::from(status_text))
        .style(Style::default().bg(theme.background));
    frame.render_widget(status_p, status_pane);
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
    active_theme: &str,
    active_lang: &str,
    active_font: &str,
    active_shell: &str,
    active_editor: &str,
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
            active_theme,
            active_lang,
            active_font,
            active_shell,
            active_editor,
            status_msg,
        );
        return;
    }

    let is_zh = active_lang == "zh";

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
            
            // Build Title span
            let prefix = if is_selected { "▶ " } else { "  " };
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
    frame.render_stateful_widget(list_widget, left_area, list_state);

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
