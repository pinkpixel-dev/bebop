use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::audio::{AudioFrame, PlaybackState};
use crate::library::format_time;
use crate::player::PlayerState;
use crate::theme::Theme;
use crate::ui::layout::AppLayout;
use crate::visualizers::Visualizer;

#[derive(Clone, Debug)]
pub struct HitZone {
    pub rect: Rect,
    pub action: HitAction,
}

#[derive(Clone, Debug, PartialEq)]
pub enum HitAction {
    TabPlayer,
    TabLibrary,
    TabQueue,
    TabHelp,
    VisualizerCycle,
    FullscreenToggle,
    SeekRatio(f32),
    VolumeRatio(f32),
    PreviousTrack,
    PlayPause,
    NextTrack,
    ToggleRepeat,
    ToggleShuffle,
    SearchSelect(usize),
    SearchClose,
}

pub struct PlayerView;

impl PlayerView {
    pub fn render(
        frame: &mut Frame,
        layout: &AppLayout,
        player: &PlayerState,
        visualizer: &mut Box<dyn Visualizer>,
        audio_frame: &AudioFrame,
        theme: &Theme,
        hit_zones: &mut Vec<HitZone>,
    ) {
        // 1. Header Bar
        Self::render_header(frame, layout.header, theme, hit_zones);

        // 2. Artwork Area
        if let Some(art_area) = layout.artwork {
            Self::render_artwork_box(frame, art_area, player, theme, hit_zones);
        }

        // 3. Visualizer Area
        Self::render_visualizer(frame, layout.visualizer, visualizer, audio_frame, theme, hit_zones);

        // 4. Player Controls Area
        Self::render_controls(frame, layout.player_controls, player, theme, hit_zones);

        // 5. Status Bar
        Self::render_status(frame, layout.status, player, theme, hit_zones);
    }

    fn render_header(frame: &mut Frame, area: Rect, theme: &Theme, hit_zones: &mut Vec<HitZone>) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.height < 1 {
            return;
        }

        let app_title = Line::from(vec![
            Span::styled(" ♪ ", Style::default().fg(theme.visualizer_primary).add_modifier(Modifier::BOLD)),
            Span::styled("Auri", Style::default().fg(theme.text).add_modifier(Modifier::BOLD)),
        ]);

        let nav_tabs = Line::from(vec![
            Span::styled("[Player] ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(" Library  ", Style::default().fg(theme.text_muted)),
            Span::styled(" Queue  ", Style::default().fg(theme.text_muted)),
            Span::styled(" ? Help ", Style::default().fg(theme.text_dim)),
        ]);

        // Render title on left
        frame.render_widget(Paragraph::new(app_title).alignment(Alignment::Left), inner);
        // Render navigation tabs on right
        frame.render_widget(Paragraph::new(nav_tabs).alignment(Alignment::Right), inner);

        // Record hit zones for top navigation tabs
        if inner.width > 35 {
            let right_start = inner.right().saturating_sub(32);
            hit_zones.push(HitZone {
                rect: Rect::new(right_start, inner.top(), 9, 1),
                action: HitAction::TabPlayer,
            });
            hit_zones.push(HitZone {
                rect: Rect::new(right_start + 9, inner.top(), 10, 1),
                action: HitAction::TabLibrary,
            });
            hit_zones.push(HitZone {
                rect: Rect::new(right_start + 19, inner.top(), 8, 1),
                action: HitAction::TabQueue,
            });
            hit_zones.push(HitZone {
                rect: Rect::new(right_start + 27, inner.top(), 8, 1),
                action: HitAction::TabHelp,
            });
        }
    }

    fn render_artwork_box(frame: &mut Frame, area: Rect, player: &PlayerState, theme: &Theme, hit_zones: &mut Vec<HitZone>) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        hit_zones.push(HitZone {
            rect: area,
            action: HitAction::FullscreenToggle,
        });

        if inner.height < 2 || inner.width < 4 {
            return;
        }

        // Placeholder graphic when Kitty image is absent or unsupported
        let center_y = inner.top() + (inner.height / 2);
        let mut lines = Vec::new();

        if let Some(track) = &player.current_track {
            if !track.has_artwork {
                lines.push(Line::from(Span::styled("  ◎  ", Style::default().fg(theme.text_dim))));
                lines.push(Line::from(Span::styled("No Artwork", Style::default().fg(theme.text_dim))));
            }
        } else {
            lines.push(Line::from(Span::styled("  ◎  ", Style::default().fg(theme.text_dim))));
            lines.push(Line::from(Span::styled("Auri Player", Style::default().fg(theme.text_dim))));
        }

        if !lines.is_empty() {
            let offset_y = center_y.saturating_sub(1);
            let display_area = Rect::new(inner.left(), offset_y, inner.width, 2.min(inner.bottom().saturating_sub(offset_y)));
            frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), display_area);
        }
    }

    fn render_visualizer(
        frame: &mut Frame,
        area: Rect,
        visualizer: &mut Box<dyn Visualizer>,
        audio_frame: &AudioFrame,
        theme: &Theme,
        hit_zones: &mut Vec<HitZone>,
    ) {
        let title = format!(" {} (v) ", visualizer.name());
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .title(Span::styled(title, Style::default().fg(theme.text_dim)))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        visualizer.render(frame, inner, audio_frame, theme);

        hit_zones.push(HitZone {
            rect: area,
            action: HitAction::VisualizerCycle,
        });
    }

    fn render_controls(
        frame: &mut Frame,
        area: Rect,
        player: &PlayerState,
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

        if inner.height < 5 {
            return;
        }

        let top_y = inner.top();

        // 1. Title & Artist lines
        let (title_text, artist_text) = if let Some(track) = &player.current_track {
            (track.title.as_str(), format!("{} • {}", track.artist, track.album))
        } else {
            ("Ready to play", "Open a file or drop a folder to begin".to_string())
        };

        let title_line = Line::from(Span::styled(
            format!("  {}", title_text),
            Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
        ));
        let artist_line = Line::from(Span::styled(
            format!("  {}", artist_text),
            Style::default().fg(theme.text_muted),
        ));

        frame.render_widget(
            Paragraph::new(title_line),
            Rect::new(inner.left(), top_y, inner.width, 1),
        );
        frame.render_widget(
            Paragraph::new(artist_line),
            Rect::new(inner.left(), top_y + 1, inner.width, 1),
        );

        // 2. Timeline Progress Bar: "02:41  ━━━━━●━━━━  04:12"
        let pos_str = format_time(player.position);
        let dur_str = format_time(player.duration);
        let timeline_y = top_y + 3;

        let left_pad = 2u16;
        let right_pad = 2u16;
        let time_width = (pos_str.len() as u16).max(5);
        let total_avail = inner.width.saturating_sub(left_pad + right_pad + (time_width * 2) + 4);

        if total_avail >= 10 {
            let ratio = if player.duration.as_secs_f32() > 0.0 {
                (player.position.as_secs_f32() / player.duration.as_secs_f32()).clamp(0.0, 1.0)
            } else {
                0.0
            };

            let filled_chars = (ratio * total_avail as f32).round() as u16;
            let bar_start_x = inner.left() + left_pad + time_width + 2;

            let mut spans = Vec::new();
            spans.push(Span::styled(format!("  {}  ", pos_str), Style::default().fg(theme.text)));

            for i in 0..total_avail {
                if i < filled_chars.saturating_sub(1) {
                    spans.push(Span::styled("━", Style::default().fg(theme.progress_elapsed)));
                } else if i == filled_chars.saturating_sub(1) || (filled_chars == 0 && i == 0) {
                    spans.push(Span::styled("●", Style::default().fg(theme.progress_thumb).add_modifier(Modifier::BOLD)));
                } else {
                    spans.push(Span::styled("━", Style::default().fg(theme.progress_remaining)));
                }
            }

            spans.push(Span::styled(format!("  {}", dur_str), Style::default().fg(theme.text_muted)));

            frame.render_widget(
                Paragraph::new(Line::from(spans)),
                Rect::new(inner.left(), timeline_y, inner.width, 1),
            );

            // Register touch/mouse seek hit zone
            hit_zones.push(HitZone {
                rect: Rect::new(bar_start_x, timeline_y, total_avail, 1),
                action: HitAction::SeekRatio(0.0), // Ratio will be computed on click
            });
        }

        // 3. Transport Buttons & Volume: "◀◀        ▶ / ❚❚        ▶▶          VOL ━━━━━━━● 80%"
        let controls_y = top_y + 4;
        let play_icon = match player.playback_state {
            PlaybackState::Playing => "❚❚ Pause",
            _ => "▶ Play ",
        };

        let vol_val = if player.muted { 0.0 } else { player.volume };
        let vol_pct = (vol_val * 100.0).round() as u16;
        let vol_bar_width = 10u16;
        let vol_filled = ((vol_val * vol_bar_width as f32).round() as u16).min(vol_bar_width);

        let mut vol_spans = Vec::new();
        vol_spans.push(Span::styled("VOL ", Style::default().fg(theme.text_muted)));
        for i in 0..vol_bar_width {
            if i < vol_filled.saturating_sub(1) {
                vol_spans.push(Span::styled("━", Style::default().fg(theme.accent)));
            } else if i == vol_filled.saturating_sub(1) || (vol_filled == 0 && i == 0) {
                vol_spans.push(Span::styled("●", Style::default().fg(theme.accent)));
            } else {
                vol_spans.push(Span::styled("━", Style::default().fg(theme.progress_remaining)));
            }
        }
        vol_spans.push(Span::styled(format!(" {}%", vol_pct), Style::default().fg(theme.text_muted)));

        let transport_line = Line::from(vec![
            Span::styled("    ◀◀ Previous  ", Style::default().fg(theme.text)),
            Span::styled(format!("  {}  ", play_icon), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("  Next ▶▶    ", Style::default().fg(theme.text)),
        ]);

        frame.render_widget(
            Paragraph::new(transport_line).alignment(Alignment::Left),
            Rect::new(inner.left(), controls_y, inner.width / 2 + 10, 1),
        );

        frame.render_widget(
            Paragraph::new(Line::from(vol_spans)).alignment(Alignment::Right),
            Rect::new(inner.left() + inner.width / 2, controls_y, inner.width / 2, 1),
        );

        // Register hit zones for buttons
        hit_zones.push(HitZone {
            rect: Rect::new(inner.left() + 4, controls_y, 14, 1),
            action: HitAction::PreviousTrack,
        });
        hit_zones.push(HitZone {
            rect: Rect::new(inner.left() + 19, controls_y, 12, 1),
            action: HitAction::PlayPause,
        });
        hit_zones.push(HitZone {
            rect: Rect::new(inner.left() + 32, controls_y, 13, 1),
            action: HitAction::NextTrack,
        });

        // Register hit zone for volume bar
        let vol_start_x = inner.right().saturating_sub(18);
        hit_zones.push(HitZone {
            rect: Rect::new(vol_start_x, controls_y, vol_bar_width, 1),
            action: HitAction::VolumeRatio(0.0),
        });
    }

    fn render_status(frame: &mut Frame, area: Rect, player: &PlayerState, theme: &Theme, hit_zones: &mut Vec<HitZone>) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.height < 1 {
            return;
        }

        let tech_details = if let Some(track) = &player.current_track {
            track.formatted_tech_details()
        } else {
            "Audio Engine Ready".to_string()
        };

        let repeat_str = player.repeat.label();
        let shuffle_str = if player.shuffle { "Shuffle On" } else { "Shuffle Off" };

        let left_line = Line::from(Span::styled(format!("  {}", tech_details), Style::default().fg(theme.text_dim)));
        let right_line = Line::from(vec![
            Span::styled(format!("{}    ", repeat_str), Style::default().fg(theme.text_muted)),
            Span::styled(format!("{}  ", shuffle_str), Style::default().fg(theme.text_muted)),
        ]);

        frame.render_widget(Paragraph::new(left_line).alignment(Alignment::Left), inner);
        frame.render_widget(Paragraph::new(right_line).alignment(Alignment::Right), inner);

        // Register hit zones for repeat and shuffle clicks
        if inner.width > 30 {
            let right_start = inner.right().saturating_sub(26);
            hit_zones.push(HitZone {
                rect: Rect::new(right_start, inner.top(), 12, 1),
                action: HitAction::ToggleRepeat,
            });
            hit_zones.push(HitZone {
                rect: Rect::new(right_start + 13, inner.top(), 13, 1),
                action: HitAction::ToggleShuffle,
            });
        }
    }
}
