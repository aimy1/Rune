use crate::core::plugin::SearchResult;
use crate::ui::ThemeStyles;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

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

fn get_nerd_font_icon(icon_name: &str) -> &'static str {
    let lower = icon_name.to_lowercase();
    if lower.contains("firefox") {
        ""
    } else if lower.contains("chrome") || lower.contains("chromium") || lower.contains("google") {
        ""
    } else if lower.contains("terminal") || lower.contains("console") || lower.contains("kitty") || lower.contains("alacritty") {
        ""
    } else if lower.contains("nvim") || lower.contains("neovim") || lower.contains("vim") {
        ""
    } else if lower.contains("code") || lower.contains("vscode") {
        "󰨞"
    } else if lower.contains("files") || lower.contains("folder") || lower.contains("nautilus") {
        "📁"
    } else if lower.contains("spotify") || lower.contains("music") {
        ""
    } else if lower.contains("vlc") || lower.contains("video") || lower.contains("player") {
        "󰕼"
    } else if lower.contains("settings") || lower.contains("control-center") || lower.contains("gear") {
        "⚙️"
    } else if lower.contains("discord") {
        "󰙯"
    } else if lower.contains("steam") {
        "󰓓"
    } else if lower.contains("git") {
        ""
    } else if lower.contains("docker") {
        "🐳"
    } else if lower.contains("python") {
        ""
    } else if lower.contains("rust") {
        ""
    } else if lower.contains("mail") || lower.contains("thunderbird") {
        "✉️"
    } else if lower.contains("chat") || lower.contains("message") || lower.contains("telegram") {
        "💬"
    } else if lower.contains("image") || lower.contains("gimp") || lower.contains("photo") {
        "🖼️"
    } else {
        "󰀻" // Default app logo
    }
}

fn get_plugin_icon(res: &SearchResult) -> &'static str {
    match res.plugin_id {
        "applications" => {
            if let Some(icon_name) = res.metadata.get("icon") {
                get_nerd_font_icon(icon_name)
            } else {
                "󰀻"
            }
        }
        "files" => {
            let path = res.metadata.get("path").map(|s| s.as_str()).unwrap_or("");
            if path.ends_with('/') {
                "📁"
            } else if path.contains('.') {
                let ext = path.split('.').last().unwrap_or("").to_lowercase();
                match ext.as_str() {
                    "rs" => "",
                    "py" => "",
                    "js" | "ts" => "",
                    "sh" | "bash" => "🐚",
                    "md" => "📝",
                    "toml" | "json" | "yaml" | "yml" => "⚙️",
                    "png" | "jpg" | "jpeg" | "gif" | "svg" => "🖼️",
                    _ => "📄"
                }
            } else {
                "📄"
            }
        }
        "calculator" => "🧮",
        "unit_converter" => "󰶱",
        "ssh" => "🔑",
        "clipboard" => "📋",
        "git" => "",
        "docker" => "🐳",
        "systemd" => "⚙️",
        "ai" => "🤖",
        "commands" => "🐚",
        _ => "🔌",
    }
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

    // Main window outline container block
    let outer_block = Block::default()
        .title(Span::styled(
            format!(" Rune Launcher [{active_plugin_name}] "),
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
    let search_block = Block::default()
        .title(Span::styled(" Search Query ", Style::default().fg(theme.border)))
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
            
            // Build Title span with Nerd Font logo/icon
            let prefix = if is_selected { "▶ " } else { "  " };
            let icon = get_plugin_icon(res);
            
            let icon_span = Span::styled(
                format!("{prefix}{icon}  "),
                Style::default().fg(if is_selected { theme.accent } else { theme.border }),
            );
            let title_span = Span::styled(
                res.title.clone(),
                Style::default().fg(if is_selected { theme.accent } else { theme.foreground }),
            );
            
            // Subtitle or plugin indicator
            let mut line_spans = vec![icon_span, title_span];
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

    let results_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(
            format!(" Matches (Found {}) ", results.len()),
            Style::default().fg(theme.border),
        ));

    let list_widget = List::new(items_list)
        .block(results_block)
        .style(Style::default().bg(theme.background));

    // Stateful render to enable list auto-scrolling
    frame.render_stateful_widget(list_widget, left_area, list_state);

    // Render Preview Box (if available)
    if let Some((r_area, text)) = right_area {
        let title_str = if preview_scroll > 0 {
            format!(" Detail Preview [Row {}] ", preview_scroll)
        } else {
            " Detail Preview ".to_string()
        };

        let preview_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title(Span::styled(title_str, Style::default().fg(theme.border)));

        let mut lines = Vec::new();
        let mut in_code_block = false;

        for line in text.lines() {
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
            " Enter Launch │ Tab Mode │ Shift-↑/↓ Scroll Detail │ Esc Exit ",
            Style::default().fg(theme.border),
        )
    } else {
        Span::styled(
            " Enter Launch │ Tab Mode │ Esc Exit ",
            Style::default().fg(theme.border),
        )
    };

    let footer_p = Paragraph::new(Line::from(vec![
        footer_text,
        Span::raw(" | "),
        Span::styled(
            format!("Themes: {total_plugins_count} active plugins"),
            Style::default().fg(theme.border).add_modifier(Modifier::DIM),
        ),
    ]))
    .style(Style::default().bg(theme.background));

    frame.render_widget(footer_p, chunks[2]);
}
