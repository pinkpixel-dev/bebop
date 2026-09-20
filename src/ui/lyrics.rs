use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::audio::PlaybackState;
use crate::library::format_time;
use crate::lyrics::{Lyrics, LyricsSource, LyricsState};
use crate::player::PlayerState;
use crate::theme::Theme;
use crate::ui::player::{HitAction, HitZone};

pub struct LyricsView;

impl LyricsView {
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        player: &PlayerState,
        state: &mut LyricsState,
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

        let deck_height = if inner.height >= 24 { 6 } else { 4 };

        let vert_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),               // Header Bar
                Constraint::Min(4),                  // Synchronized Lyrics Body
                Constraint::Length(deck_height),     // Bottom Controls / Status Deck
            ])
            .split(inner);

        let header_area = vert_chunks[0];
        let lyrics_area = vert_chunks[1];
        let footer_area = vert_chunks[2];

        // 1. Header Bar
        Self::render_header(frame, header_area, player, state.lyrics.as_ref(), theme, hit_zones);

        // 2. Lyrics Main Body
        Self::render_lyrics_body(frame, lyrics_area, player, state, theme, hit_zones);

        // 3. Mini Transport / Status Deck
        Self::render_bottom_deck(frame, footer_area, player, theme, hit_zones);
    }

    fn render_header(
        frame: &mut Frame,
        area: Rect,
        player: &PlayerState,
        lyrics: Option<&Lyrics>,
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

        if inner.height < 1 {
            return;
        }

        let (title_text, source_badge) = if let Some(track) = &player.current_track {
            let badge = match lyrics.map(|l| &l.source) {
                Some(LyricsSource::CompanionFile(_)) => " [LRC File] ",
                Some(LyricsSource::EmbeddedTag) => " [Embedded] ",
                _ => " [No LRC] ",
            };
            (format!("{} • {}", track.title, track.artist), badge)
        } else {
            ("No Track Playing".to_string(), "")
        };

        let header_title = Line::from(vec![
            Span::styled(" ♪ Bebop  ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(title_text, Style::default().fg(theme.text).add_modifier(Modifier::BOLD)),
            Span::styled(source_badge, Style::default().fg(theme.visualizer_primary)),
        ]);

        let nav_tabs = Line::from(vec![
            Span::styled(" Player  ", Style::default().fg(theme.text_muted)),
            Span::styled(" Library  ", Style::default().fg(theme.text_muted)),
            Span::styled(" Queue  ", Style::default().fg(theme.text_muted)),
            Span::styled("[Lyrics] ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(" ? Help ", Style::default().fg(theme.text_dim)),
        ]);

        frame.render_widget(Paragraph::new(header_title).alignment(Alignment::Left), inner);
        frame.render_widget(Paragraph::new(nav_tabs).alignment(Alignment::Right), inner);

        // Header hit zones for tabs
        if inner.width > 42 {
            let right_start = inner.right().saturating_sub(40);
            hit_zones.push(HitZone {
                rect: Rect::new(right_start, inner.top(), 9, 1),
                action: HitAction::TabPlayer,
            });
            hit_zones.push(HitZone {
                rect: Rect::new(right_start + 9, inner.top(), 9, 1),
                action: HitAction::TabLibrary,
            });
            hit_zones.push(HitZone {
                rect: Rect::new(right_start + 18, inner.top(), 7, 1),
                action: HitAction::TabQueue,
            });
            hit_zones.push(HitZone {
                rect: Rect::new(right_start + 25, inner.top(), 8, 1),
                action: HitAction::TabLyrics,
            });
            hit_zones.push(HitZone {
                rect: Rect::new(right_start + 33, inner.top(), 7, 1),
                action: HitAction::TabHelp,
            });
        }
    }

    fn render_lyrics_body(
        frame: &mut Frame,
        area: Rect,
        player: &PlayerState,
        state: &mut LyricsState,
        theme: &Theme,
        hit_zones: &mut Vec<HitZone>,
    ) {
        let auto_scroll_text = if !state.auto_scroll {
            " [Auto-scroll paused · press 's' or Enter to re-sync] "
        } else {
            " [Synchronized] "
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .title(Span::styled(" Synchronized Lyrics (l) ", Style::default().fg(theme.text_dim)))
            .title_alignment(Alignment::Left)
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Status indicator in top-right
        if inner.height > 1 && inner.width > 50 {
            let status_span = Span::styled(
                auto_scroll_text,
                if state.auto_scroll {
                    Style::default().fg(theme.text_dim)
                } else {
                    Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)
                },
            );
            let status_rect = Rect::new(
                inner.right().saturating_sub(auto_scroll_text.len() as u16 + 1),
                area.top(),
                auto_scroll_text.len() as u16,
                1,
            );
            frame.render_widget(Paragraph::new(status_span), status_rect);
        }

        if inner.height < 2 {
            return;
        }

        let active_idx = state.lyrics.as_ref().and_then(|l| l.find_active_index(player.position));
        if state.auto_scroll {
            if let Some(idx) = active_idx {
                state.selected_line_idx = idx;
            }
        }

        // Empty state when lyrics are missing
        let lyrics = match state.lyrics.as_ref() {
            Some(l) if !l.lines.is_empty() => l,
            _ => {
                Self::render_empty_state(frame, inner, player, theme);
                return;
            }
        };

        let total_lines = lyrics.lines.len();
        let avail_rows = inner.height as usize;
        let center_offset = avail_rows / 2;

        let target_idx = if state.auto_scroll {
            active_idx.unwrap_or(0)
        } else {
            state.selected_line_idx
        };

        let start_line_idx = target_idx.saturating_sub(center_offset);
        let current_pos = player.position;

        for row in 0..avail_rows {
            let line_idx = start_line_idx + row;
            if line_idx >= total_lines {
                break;
            }

            let lyric_line = &lyrics.lines[line_idx];
            let is_active = Some(line_idx) == active_idx;
            let is_selected_manual = !state.auto_scroll && line_idx == state.selected_line_idx;
            let is_past = lyric_line.timestamp < current_pos && !is_active;

            let row_y = inner.top() + row as u16;
            let row_rect = Rect::new(inner.left(), row_y, inner.width, 1);

            // Time string: [01:23]
            let ts_str = format!("[{}] ", format_time(lyric_line.timestamp));

            let spans = if is_active {
                vec![
                    Span::styled(" ▶ ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                    Span::styled(ts_str, Style::default().fg(theme.accent)),
                    Span::styled(
                        &lyric_line.text,
                        Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
                    ),
                ]
            } else if is_selected_manual {
                vec![
                    Span::styled(" › ", Style::default().fg(theme.visualizer_primary).add_modifier(Modifier::BOLD)),
                    Span::styled(ts_str, Style::default().fg(theme.text_dim)),
                    Span::styled(&lyric_line.text, Style::default().fg(theme.text)),
                ]
            } else if is_past {
                vec![
                    Span::styled("   ", Style::default()),
                    Span::styled(ts_str, Style::default().fg(theme.text_dim)),
                    Span::styled(&lyric_line.text, Style::default().fg(theme.text_dim)),
                ]
            } else {
                vec![
                    Span::styled("   ", Style::default()),
                    Span::styled(ts_str, Style::default().fg(theme.text_dim)),
                    Span::styled(&lyric_line.text, Style::default().fg(theme.text)),
                ]
            };

            frame.render_widget(Paragraph::new(Line::from(spans)), row_rect);

            // Register hit zone for tap/click to seek to this line
            hit_zones.push(HitZone {
                rect: row_rect,
                action: HitAction::SeekLyric(lyric_line.timestamp),
            });
        }
    }

    fn render_empty_state(frame: &mut Frame, area: Rect, player: &PlayerState, theme: &Theme) {
        let (song_name, stem_name) = if let Some(track) = &player.current_track {
            let stem = track.path.file_stem().and_then(|s| s.to_str()).unwrap_or("song");
            (track.title.as_str(), stem)
        } else {
            ("No Track", "track")
        };

        let lines = vec![
            Line::from(Span::styled("  ♪  ", Style::default().fg(theme.text_dim))),
            Line::from(Span::styled(
                "No Synchronized Lyrics (.lrc) Found",
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!("For: {}", song_name),
                Style::default().fg(theme.text_muted),
            )),
            Line::from(Span::raw("")),
            Line::from(Span::styled(
                "Place a companion LRC file next to your audio track:",
                Style::default().fg(theme.text_dim),
            )),
            Line::from(Span::styled(
                format!("  {}.lrc", stem_name),
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::raw("")),
            Line::from(Span::styled(
                "Or embed synchronized/unsynchronized lyrics tags inside the audio file.",
                Style::default().fg(theme.text_dim),
            )),
        ];

        let center_y = area.top() + (area.height.saturating_sub(lines.len() as u16) / 2);
        let display_area = Rect::new(area.left(), center_y, area.width, lines.len() as u16);
        frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), display_area);
    }

    fn render_bottom_deck(
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

        if inner.height < 2 {
            return;
        }

        let top_y = inner.top();

        // 1. Timeline Progress Bar: "02:41  ━━━━━●━━━━  04:12"
        let pos_str = format_time(player.position);
        let dur_str = format_time(player.duration);
        let timeline_y = top_y;

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

            hit_zones.push(HitZone {
                rect: Rect::new(bar_start_x, timeline_y, total_avail, 1),
                action: HitAction::SeekRatio(0.0),
            });
        }

        // 2. Transport & volume controls row (if space permits)
        if inner.height >= 3 {
            let controls_y = top_y + 1;
            let play_icon = match player.playback_state {
                PlaybackState::Playing => "❚❚ Pause",
                _ => "▶ Play ",
            };

            let vol_val = if player.muted { 0.0 } else { player.volume };
            let vol_pct = (vol_val * 100.0).round() as u16;
            let vol_bar_width = 8u16;
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
                Span::styled("  ◀◀ Prev  ", Style::default().fg(theme.text)),
                Span::styled(format!(" {} ", play_icon), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled(" Next ▶▶  ", Style::default().fg(theme.text)),
                Span::styled("│ Enter: Seek  s: Sync  Esc/l: Player", Style::default().fg(theme.text_dim)),
            ]);

            frame.render_widget(
                Paragraph::new(transport_line).alignment(Alignment::Left),
                Rect::new(inner.left(), controls_y, inner.width / 2 + 15, 1),
            );

            frame.render_widget(
                Paragraph::new(Line::from(vol_spans)).alignment(Alignment::Right),
                Rect::new(inner.left() + inner.width / 2, controls_y, inner.width / 2, 1),
            );

            // Register hit zones for transport buttons
            hit_zones.push(HitZone {
                rect: Rect::new(inner.left() + 2, controls_y, 11, 1),
                action: HitAction::PreviousTrack,
            });
            hit_zones.push(HitZone {
                rect: Rect::new(inner.left() + 13, controls_y, 10, 1),
                action: HitAction::PlayPause,
            });
            hit_zones.push(HitZone {
                rect: Rect::new(inner.left() + 23, controls_y, 11, 1),
                action: HitAction::NextTrack,
            });

            // Register hit zone for volume
            let vol_start_x = inner.right().saturating_sub(16);
            hit_zones.push(HitZone {
                rect: Rect::new(vol_start_x, controls_y, vol_bar_width, 1),
                action: HitAction::VolumeRatio(0.0),
            });
        }
    }
}
