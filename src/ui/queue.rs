use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
    ScrollbarState,
};
use ratatui::Frame;

use crate::player::Queue;
use crate::theme::Theme;
use crate::ui::player::{HitAction, HitZone};

#[derive(Clone, Debug, Default)]
pub struct QueueState {
    pub selected_idx: usize,
    pub scroll_offset: usize,
}

impl QueueState {
    pub fn new() -> Self {
        Self {
            selected_idx: 0,
            scroll_offset: 0,
        }
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

    pub fn page_up(&mut self, page_size: usize) {
        self.selected_idx = self.selected_idx.saturating_sub(page_size);
    }

    pub fn page_down(&mut self, page_size: usize, total: usize) {
        if total > 0 {
            self.selected_idx = (self.selected_idx + page_size).min(total - 1);
        }
    }

    pub fn jump_to_start(&mut self) {
        self.selected_idx = 0;
    }

    pub fn jump_to_end(&mut self, total: usize) {
        if total > 0 {
            self.selected_idx = total - 1;
        }
    }

    pub fn ensure_visible(&mut self, visible_height: usize, total: usize) {
        if visible_height == 0 || total == 0 {
            self.scroll_offset = 0;
            self.selected_idx = 0;
            return;
        }
        if self.selected_idx >= total {
            self.selected_idx = total - 1;
        }
        if self.selected_idx < self.scroll_offset {
            self.scroll_offset = self.selected_idx;
        } else if self.selected_idx >= self.scroll_offset + visible_height {
            self.scroll_offset = self.selected_idx + 1 - visible_height;
        }
        let max_offset = total.saturating_sub(visible_height);
        if self.scroll_offset > max_offset {
            self.scroll_offset = max_offset;
        }
    }
}

pub struct QueueView;

impl QueueView {
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        queue: &Queue,
        state: &mut QueueState,
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
        let shuffle_status = if queue.shuffle { " [Shuffle: On]" } else { "" };
        let header_title = Line::from(vec![
            Span::styled(" ♪ Bebop  ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(format!("[Playback Queue] ({} tracks){}", queue.tracks.len(), shuffle_status), Style::default().fg(theme.text).add_modifier(Modifier::BOLD)),
        ]);
        let nav_tabs = Line::from(vec![
            Span::styled(" Player  ", Style::default().fg(theme.text_muted)),
            Span::styled(" Library  ", Style::default().fg(theme.text_muted)),
            Span::styled("[Queue] ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(" Lyrics  ", Style::default().fg(theme.text_muted)),
            Span::styled(" ? Help ", Style::default().fg(theme.text_dim)),
        ]);
        frame.render_widget(Paragraph::new(header_title).alignment(Alignment::Left), header_area);
        frame.render_widget(Paragraph::new(nav_tabs).alignment(Alignment::Right), header_area);

        // Header tab hit zones
        if header_area.width > 42 {
            let right_start = header_area.right().saturating_sub(40);
            hit_zones.push(HitZone {
                rect: Rect::new(right_start, header_area.top(), 9, 1),
                action: HitAction::TabPlayer,
            });
            hit_zones.push(HitZone {
                rect: Rect::new(right_start + 9, header_area.top(), 9, 1),
                action: HitAction::TabLibrary,
            });
            hit_zones.push(HitZone {
                rect: Rect::new(right_start + 18, header_area.top(), 7, 1),
                action: HitAction::TabQueue,
            });
            hit_zones.push(HitZone {
                rect: Rect::new(right_start + 25, header_area.top(), 8, 1),
                action: HitAction::TabLyrics,
            });
            hit_zones.push(HitZone {
                rect: Rect::new(right_start + 33, header_area.top(), 7, 1),
                action: HitAction::TabHelp,
            });
        }

        // 2. Queue list
        let visible_height = list_area.height.saturating_sub(2) as usize;
        state.ensure_visible(visible_height, queue.tracks.len());

        let mut list_items = Vec::new();
        if queue.tracks.is_empty() {
            list_items.push(ListItem::new("  (Queue is currently empty. Press '2' to browse Library and add tracks)").style(Style::default().fg(theme.text_dim)));
        } else {
            let visible_tracks = queue
                .tracks
                .iter()
                .enumerate()
                .skip(state.scroll_offset)
                .take(visible_height);

            for (i, t) in visible_tracks {
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

                // Hit zone for tap / click selection
                let row_offset = (i - state.scroll_offset) as u16;
                let row_y = list_area.top() + 1 + row_offset;
                if row_y < list_area.bottom().saturating_sub(1) {
                    hit_zones.push(HitZone {
                        rect: Rect::new(list_area.left() + 1, row_y, list_area.width.saturating_sub(2), 1),
                        action: HitAction::QueueSelect(i),
                    });
                }
            }
        }

        let mut queue_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border));

        if queue.tracks.len() > visible_height && !queue.tracks.is_empty() {
            queue_block = queue_block.title(
                Span::styled(
                    format!(" [{}/{} tracks] ", state.selected_idx + 1, queue.tracks.len()),
                    Style::default().fg(theme.text_dim),
                )
            );
        }

        let queue_list = List::new(list_items).block(queue_block);
        frame.render_widget(queue_list, list_area);

        // Render scrollbar if list exceeds visible height
        if queue.tracks.len() > visible_height {
            let mut scrollbar_state = ScrollbarState::new(queue.tracks.len())
                .position(state.selected_idx)
                .viewport_content_length(visible_height);
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .end_symbol(None)
                    .track_symbol(Some("│"))
                    .thumb_symbol("█")
                    .style(Style::default().fg(theme.text_dim)),
                list_area,
                &mut scrollbar_state,
            );
        }

        // 3. Footer instructions
        let shuffle_lbl = if queue.shuffle { "Shuffle [On]  " } else { "Shuffle [Off]  " };
        let tips = Line::from(vec![
            Span::styled("Enter: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("Jump to Track  ", Style::default().fg(theme.text_muted)),
            Span::styled("d / Del: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("Remove  ", Style::default().fg(theme.text_muted)),
            Span::styled("c: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("Clear  ", Style::default().fg(theme.text_muted)),
            Span::styled("s: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(shuffle_lbl, Style::default().fg(if queue.shuffle { theme.visualizer_primary } else { theme.text_muted })),
            Span::styled("PgUp/PgDn: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("Scroll  ", Style::default().fg(theme.text_muted)),
            Span::styled("1: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("Player View", Style::default().fg(theme.text_muted)),
        ]);
        frame.render_widget(Paragraph::new(tips).alignment(Alignment::Center), footer_area);

        let total_tip_len = 76u16;
        if footer_area.width >= total_tip_len {
            let start_x = footer_area.x + (footer_area.width - total_tip_len) / 2;
            hit_zones.push(HitZone {
                rect: Rect::new(start_x + 49, footer_area.top(), 16, 1),
                action: HitAction::ToggleShuffle,
            });
        }
    }
}
