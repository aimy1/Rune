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
