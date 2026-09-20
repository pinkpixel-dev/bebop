use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::theme::Theme;

pub struct HelpOverlay;

impl HelpOverlay {
    pub fn render(frame: &mut Frame, area: Rect, theme: &Theme) {
        let popup_width = (area.width * 7 / 10).clamp(40, 70);
        let popup_height = (area.height * 8 / 10).clamp(18, 28);

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

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_focused))
            .title(Span::styled(" Auri Keybindings & Touch Controls ", Style::default().fg(theme.text).add_modifier(Modifier::BOLD)))
            .style(Style::default().bg(theme.surface));

        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        let lines = vec![
            Line::from(Span::styled("Playback Controls", Style::default().fg(theme.visualizer_primary).add_modifier(Modifier::BOLD))),
            Line::from(vec![
                Span::styled("  Space          ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled("Play / Pause (or tap transport button)", Style::default().fg(theme.text_muted)),
            ]),
            Line::from(vec![
                Span::styled("  n / p          ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled("Next / Previous track", Style::default().fg(theme.text_muted)),
            ]),
            Line::from(vec![
                Span::styled("  ← / →          ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled("Seek ±5s (or tap progress bar)", Style::default().fg(theme.text_muted)),
            ]),
            Line::from(vec![
                Span::styled("  ↑ / ↓          ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled("Volume ±5% (or tap volume bar)", Style::default().fg(theme.text_muted)),
            ]),
            Line::from(vec![
                Span::styled("  m              ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled("Mute / Unmute", Style::default().fg(theme.text_muted)),
            ]),
            Line::from(vec![
                Span::styled("  r / s          ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled("Cycle Repeat / Toggle Shuffle", Style::default().fg(theme.text_muted)),
            ]),
            Line::from(""),
            Line::from(Span::styled("Visuals & Views", Style::default().fg(theme.visualizer_primary).add_modifier(Modifier::BOLD))),
            Line::from(vec![
                Span::styled("  f              ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled("Full-screen visualizer mode", Style::default().fg(theme.text_muted)),
            ]),
            Line::from(vec![
                Span::styled("  v              ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled("Cycle visualizer (Bars / Mirrored / Waveform / VU / Waterfall / Particles)", Style::default().fg(theme.text_muted)),
            ]),
            Line::from(vec![
                Span::styled("  t              ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled("Cycle theme palette (Neon / Dark / Ice / Sunset / Candy / Matrix / Mono / Album)", Style::default().fg(theme.text_muted)),
            ]),
            Line::from(vec![
                Span::styled("  a              ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled("Toggle album artwork", Style::default().fg(theme.text_muted)),
            ]),
            Line::from(vec![
                Span::styled("  1 / 2 / 3 / 4  ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled("Player / Library / Queue / Lyrics views", Style::default().fg(theme.text_muted)),
            ]),
            Line::from(vec![
                Span::styled("  l              ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled("Toggle synchronized lyrics view", Style::default().fg(theme.text_muted)),
            ]),
            Line::from(vec![
                Span::styled("  Enter / s      ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled("Seek to lyric / Re-sync scroll (in Lyrics)", Style::default().fg(theme.text_muted)),
            ]),
            Line::from(vec![
                Span::styled("  /              ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled("Search music library", Style::default().fg(theme.text_muted)),
            ]),
            Line::from(vec![
                Span::styled("  ? / Esc        ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled("Toggle this help screen", Style::default().fg(theme.text_muted)),
            ]),
            Line::from(vec![
                Span::styled("  q / Ctrl+C     ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled("Quit Auri", Style::default().fg(theme.text_muted)),
            ]),
        ];

        frame.render_widget(Paragraph::new(lines).alignment(Alignment::Left), inner);
    }
}
