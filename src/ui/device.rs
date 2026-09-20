use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::audio::AudioDeviceInfo;
use crate::theme::Theme;
use crate::ui::player::{HitAction, HitZone};

#[derive(Clone, Debug, Default)]
pub struct DeviceState {
    pub is_open: bool,
    pub devices: Vec<AudioDeviceInfo>,
    pub selected_index: usize,
}

impl DeviceState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self, devices: Vec<AudioDeviceInfo>, active_name: Option<&str>) {
        self.devices = devices;
        self.is_open = true;

        if let Some(active) = active_name {
            if let Some(idx) = self.devices.iter().position(|d| d.name == active) {
                self.selected_index = idx;
                return;
            }
        }

        if let Some(idx) = self.devices.iter().position(|d| d.is_active) {
            self.selected_index = idx;
        } else if let Some(idx) = self.devices.iter().position(|d| d.is_default) {
            self.selected_index = idx;
        } else {
            self.selected_index = 0;
        }
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    pub fn move_up(&mut self) {
        if !self.devices.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.devices.len() - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }

    pub fn move_down(&mut self) {
        if !self.devices.is_empty() {
            if self.selected_index + 1 >= self.devices.len() {
                self.selected_index = 0;
            } else {
                self.selected_index += 1;
            }
        }
    }

    pub fn selected_device(&self) -> Option<&AudioDeviceInfo> {
        self.devices.get(self.selected_index)
    }
}

pub struct DeviceOverlay;

impl DeviceOverlay {
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        state: &DeviceState,
        theme: &Theme,
        hit_zones: &mut Vec<HitZone>,
    ) {
        let popup_width = (area.width * 75 / 100).clamp(45, 80);
        let popup_height = (area.height * 70 / 100).clamp(12, 24);

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

        // Clear background under popup to prevent visual bleed-through
        frame.render_widget(Clear, popup_area);

        let title = format!(" Audio Output Devices ({} detected) ", state.devices.len());
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_focused))
            .title(Span::styled(title, Style::default().fg(theme.text).add_modifier(Modifier::BOLD)))
            .style(Style::default().bg(theme.surface));

        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        if inner.height < 4 {
            return;
        }

        // Layout inner area: device list on top, controls hint at bottom
        let inner_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(2),
                Constraint::Length(1),
            ])
            .split(inner);

        let list_area = inner_layout[0];
        let hint_area = inner_layout[1];

        // Render device list
        if state.devices.is_empty() {
            let empty_msg = Paragraph::new(Line::from(vec![
                Span::styled("  No audio output devices detected on host audio subsystem.", Style::default().fg(theme.text_muted)),
            ])).alignment(Alignment::Left);
            frame.render_widget(empty_msg, list_area);
        } else {
            let max_visible = list_area.height as usize;
            let scroll_offset = if state.selected_index >= max_visible {
                state.selected_index - max_visible + 1
            } else {
                0
            };

            for (i, dev) in state.devices.iter().enumerate().skip(scroll_offset).take(max_visible) {
                let row_y = list_area.top() + (i - scroll_offset) as u16;
                let is_selected = i == state.selected_index;

                let mut spans = Vec::new();
                if is_selected {
                    spans.push(Span::styled(" ► ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)));
                } else {
                    spans.push(Span::raw("   "));
                }

                if dev.is_active {
                    spans.push(Span::styled("● [Active] ", Style::default().fg(theme.visualizer_primary).add_modifier(Modifier::BOLD)));
                } else {
                    spans.push(Span::styled("○          ", Style::default().fg(theme.text_dim)));
                }

                let name_style = if is_selected {
                    Style::default().fg(theme.text).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text)
                };
                spans.push(Span::styled(&dev.name, name_style));

                if dev.is_default {
                    spans.push(Span::styled(" (Default)", Style::default().fg(theme.text_muted)));
                }

                let row_line = Line::from(spans);
                let row_rect = Rect::new(list_area.left(), row_y, list_area.width, 1);

                if is_selected {
                    let highlight_block = Block::default().style(Style::default().bg(theme.border));
                    frame.render_widget(highlight_block, row_rect);
                }

                frame.render_widget(Paragraph::new(row_line), row_rect);

                // Register touch / mouse click hit zone
                hit_zones.push(HitZone {
                    rect: row_rect,
                    action: HitAction::DeviceSelect(i),
                });
            }
        }

        // Render navigation hint line
        let hint_line = Line::from(vec![
            Span::styled(" Enter/Tap", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(" Select  ", Style::default().fg(theme.text_dim)),
            Span::styled("↑/↓/k/j", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(" Navigate  ", Style::default().fg(theme.text_dim)),
            Span::styled("Esc / o", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(" Close", Style::default().fg(theme.text_dim)),
        ]);
        frame.render_widget(Paragraph::new(hint_line).alignment(Alignment::Center), hint_area);
    }
}
