use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::audio::AudioFrame;
use crate::library::format_time;
use crate::player::PlayerState;
use crate::theme::Theme;
use crate::ui::player::{HitAction, HitZone};
use crate::visualizers::Visualizer;

pub struct FullscreenView;

impl FullscreenView {
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        player: &PlayerState,
        visualizer: &mut Box<dyn Visualizer>,
        audio_frame: &AudioFrame,
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

        // Split into main visualizer area and bottom minimal now-playing strip (3 rows)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(4),
                Constraint::Length(3),
            ])
            .split(inner);

        let viz_area = chunks[0];
        let strip_area = chunks[1];

        // Render visualizer
        visualizer.render(frame, viz_area, audio_frame, theme);

        hit_zones.push(HitZone {
            rect: viz_area,
            action: HitAction::FullscreenToggle,
        });

        // Bottom now-playing strip
        let title_text = if let Some(track) = &player.current_track {
            track.title.as_str()
        } else {
            "Bebop Player"
        };

        let title_line = Line::from(Span::styled(
            title_text,
            Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
        ));

        let pos_str = format_time(player.position);
        let dur_str = format_time(player.duration);
        let bar_width = 16u16;

        let ratio = if player.duration.as_secs_f32() > 0.0 {
            (player.position.as_secs_f32() / player.duration.as_secs_f32()).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let filled_chars = (ratio * bar_width as f32).round() as u16;
        let mut progress_spans = Vec::new();
        progress_spans.push(Span::styled(format!("{} ", pos_str), Style::default().fg(theme.text_muted)));

        for i in 0..bar_width {
            if i < filled_chars.saturating_sub(1) {
                progress_spans.push(Span::styled("━", Style::default().fg(theme.progress_elapsed)));
            } else if i == filled_chars.saturating_sub(1) || (filled_chars == 0 && i == 0) {
                progress_spans.push(Span::styled("●", Style::default().fg(theme.progress_thumb).add_modifier(Modifier::BOLD)));
            } else {
                progress_spans.push(Span::styled("━", Style::default().fg(theme.progress_remaining)));
            }
        }
        progress_spans.push(Span::styled(format!(" {}", dur_str), Style::default().fg(theme.text_muted)));

        frame.render_widget(
            Paragraph::new(title_line).alignment(Alignment::Center),
            Rect::new(strip_area.left(), strip_area.top(), strip_area.width, 1),
        );

        frame.render_widget(
            Paragraph::new(Line::from(progress_spans)).alignment(Alignment::Center),
            Rect::new(strip_area.left(), strip_area.top() + 1, strip_area.width, 1),
        );
    }
}
