use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::player::Queue;
use crate::theme::Theme;
use crate::ui::player::{HitAction, HitZone};

pub struct QueueState {
    pub selected_idx: usize,
}

impl QueueState {
    pub fn new() -> Self {
        Self { selected_idx: 0 }
    }

    pub fn move_up(&mut self) {
        if self.selected_idx > 0 {
            self.selected_idx -= 1;
        }
    }

    pub fn move_down(&mut self, total: usize) {
        if total > 0 && self.selected_idx + 1 < total {
            self.selected_idx += 1;
        }
    }
}

pub struct QueueView;

impl QueueView {
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        queue: &Queue,
        state: &QueueState,
        theme: &Theme,
        hit_zones: &mut Vec<HitZone>,
    ) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.height < 6 {
            return;
        }

        let vert_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(4),
                Constraint::Length(2),
            ])
            .split(inner);

        let header_area = vert_chunks[0];
        let list_area = vert_chunks[1];
        let footer_area = vert_chunks[2];

        // 1. Header
        let header_title = Line::from(vec![
            Span::styled(" ♪ Auri  ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(format!("[Playback Queue] ({} tracks)", queue.tracks.len()), Style::default().fg(theme.text).add_modifier(Modifier::BOLD)),
        ]);
        let nav_tabs = Line::from(vec![
            Span::styled(" Player  ", Style::default().fg(theme.text_muted)),
            Span::styled(" Library  ", Style::default().fg(theme.text_muted)),
            Span::styled("[Queue] ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(" ? Help ", Style::default().fg(theme.text_dim)),
        ]);
        frame.render_widget(Paragraph::new(header_title).alignment(Alignment::Left), header_area);
        frame.render_widget(Paragraph::new(nav_tabs).alignment(Alignment::Right), header_area);

        // Header tab hit zones
        if header_area.width > 35 {
            let right_start = header_area.right().saturating_sub(32);
            hit_zones.push(HitZone {
                rect: Rect::new(right_start, header_area.top(), 9, 1),
                action: HitAction::TabPlayer,
            });
            hit_zones.push(HitZone {
                rect: Rect::new(right_start + 9, header_area.top(), 10, 1),
                action: HitAction::TabLibrary,
            });
            hit_zones.push(HitZone {
                rect: Rect::new(right_start + 19, header_area.top(), 8, 1),
                action: HitAction::TabQueue,
            });
            hit_zones.push(HitZone {
                rect: Rect::new(right_start + 27, header_area.top(), 8, 1),
                action: HitAction::TabHelp,
            });
        }

        // 2. Queue list
        let mut list_items = Vec::new();
        if queue.tracks.is_empty() {
            list_items.push(ListItem::new("  (Queue is currently empty. Press '2' to browse Library and add tracks)").style(Style::default().fg(theme.text_dim)));
        } else {
            for (i, t) in queue.tracks.iter().enumerate() {
                let is_current = queue.current_index == Some(i);
                let current_marker = if is_current { "▶ " } else { "  " };
                let dur = t.formatted_duration();
                let label = format!("{} {:02}. {:<45} {:>8}", current_marker, i + 1, t.title, dur);

                let is_selected = state.selected_idx == i;
                let style = if is_selected {
                    Style::default().fg(theme.accent).bg(theme.surface).add_modifier(Modifier::BOLD)
                } else if is_current {
                    Style::default().fg(theme.visualizer_primary).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text)
                };

                list_items.push(ListItem::new(label).style(style));
            }
        }

        let queue_list = List::new(list_items)
            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(theme.border)));
        frame.render_widget(queue_list, list_area);

        // 3. Footer instructions
        let tips = Line::from(vec![
            Span::styled("Enter: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("Jump to Track  ", Style::default().fg(theme.text_muted)),
            Span::styled("d / Del: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("Remove  ", Style::default().fg(theme.text_muted)),
            Span::styled("c: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("Clear  ", Style::default().fg(theme.text_muted)),
            Span::styled("s: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("Shuffle  ", Style::default().fg(theme.text_muted)),
            Span::styled("1: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("Player View", Style::default().fg(theme.text_muted)),
        ]);
        frame.render_widget(Paragraph::new(tips).alignment(Alignment::Center), footer_area);
    }
}
