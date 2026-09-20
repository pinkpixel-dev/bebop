use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::library::SearchState;
use crate::theme::Theme;
use crate::ui::player::{HitAction, HitZone};

pub struct SearchOverlay;

impl SearchOverlay {
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        state: &SearchState,
        theme: &Theme,
        hit_zones: &mut Vec<HitZone>,
    ) {
        let popup_width = (area.width * 8 / 10).clamp(45, 85);
        let popup_height = (area.height * 75 / 100).clamp(14, 28);

        let vert_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length((area.height.saturating_sub(popup_height)) / 2),
                Constraint::Length(popup_height),
                Constraint::Min(0),
            ])
            .split(area);

        let horiz_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length((area.width.saturating_sub(popup_width)) / 2),
                Constraint::Length(popup_width),
                Constraint::Min(0),
            ])
            .split(vert_layout[1]);

        let popup_area = horiz_layout[1];

        // Clear background under popup
        frame.render_widget(Clear, popup_area);

        let title = format!(" Search Music Library ({} matches) ", state.results.len());
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_focused))
            .title(Span::styled(title, Style::default().fg(theme.text).add_modifier(Modifier::BOLD)))
            .style(Style::default().bg(theme.surface));

        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        if inner.height < 5 {
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Input box
                Constraint::Min(4),    // Result list
                Constraint::Length(2), // Tips
            ])
            .split(inner);

        let input_area = chunks[0];
        let list_area = chunks[1];
        let footer_area = chunks[2];

        // 1. Search Query Input Box
        let query_text = if state.query.is_empty() {
            "Type title, artist, album, or filename..."
        } else {
            &state.query
        };

        let input_style = if state.query.is_empty() {
            Style::default().fg(theme.text_dim)
        } else {
            Style::default().fg(theme.text).add_modifier(Modifier::BOLD)
        };

        let cursor_span = Span::styled("█", Style::default().fg(theme.accent));
        let prompt_span = Span::styled("🔍 ", Style::default().fg(theme.accent));
        let text_span = Span::styled(query_text, input_style);

        let input_line = if state.query.is_empty() {
            Line::from(vec![prompt_span, cursor_span, Span::raw(" "), text_span])
        } else {
            Line::from(vec![prompt_span, text_span, cursor_span])
        };

        let input_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.accent))
            .style(Style::default().bg(theme.bg));

        frame.render_widget(Paragraph::new(input_line).block(input_block), input_area);

        // 2. Results List
        let mut list_items = Vec::new();
        if state.results.is_empty() {
            let msg = if state.query.is_empty() {
                "  (No tracks available to search)"
            } else {
                "  No tracks matched your search query"
            };
            list_items.push(ListItem::new(msg).style(Style::default().fg(theme.text_dim)));
        } else {
            // Scroll window to keep selected_idx visible
            let max_visible = list_area.height.saturating_sub(2) as usize;
            let start_idx = if max_visible > 0 && state.selected_idx >= max_visible {
                state.selected_idx - max_visible + 1
            } else {
                0
            };

            for (i, track) in state.results.iter().enumerate().skip(start_idx).take(max_visible.max(1)) {
                let is_selected = state.selected_idx == i;
                let marker = if is_selected { "▶ " } else { "  " };

                let dur = track.formatted_duration();
                let display_line = format!(
                    "{}{:<32} {:<24} {:>8}",
                    marker,
                    truncate_str(&track.title, 32),
                    truncate_str(&format!("{} • {}", track.artist, track.album), 24),
                    dur
                );

                let style = if is_selected {
                    Style::default().fg(theme.accent).bg(theme.surface).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text)
                };

                list_items.push(ListItem::new(display_line).style(style));

                // Hit zone for tap / mouse click on search result
                let row_offset = (i.saturating_sub(start_idx)) as u16;
                let item_y = list_area.top() + 1 + row_offset;
                if item_y < list_area.bottom() {
                    hit_zones.push(HitZone {
                        rect: Rect::new(list_area.left() + 1, item_y, list_area.width.saturating_sub(2), 1),
                        action: HitAction::SearchSelect(i),
                    });
                }
            }
        }

        let results_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.bg));

        frame.render_widget(List::new(list_items).block(results_block), list_area);

        // 3. Footer Navigation Tips
        let tips = Line::from(vec![
            Span::styled("↑ / ↓: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("Navigate  ", Style::default().fg(theme.text_muted)),
            Span::styled("Enter: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("Play Now  ", Style::default().fg(theme.text_muted)),
            Span::styled("Esc: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("Dismiss", Style::default().fg(theme.text_muted)),
        ]);
        frame.render_widget(Paragraph::new(tips).alignment(Alignment::Center), footer_area);
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len.saturating_sub(1)).collect();
        format!("{}…", truncated)
    }
}
