use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::audio::{AudioFrame, PlaybackState};
use crate::player::PlayerState;
use crate::theme::Theme;
use crate::ui::player::{HitAction, HitZone};

pub struct PetView;

impl PetView {
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        player: &PlayerState,
        audio_frame: &AudioFrame,
        theme: &Theme,
        hit_zones: &mut Vec<HitZone>,
    ) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .title(Span::styled(
                " 🐾 Kyoku ",
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Register hit zone on the pet box so clicking or tapping toggles or interacts
        hit_zones.push(HitZone {
            rect: area,
            action: HitAction::TogglePet,
        });

        if inner.height < 3 || inner.width < 6 {
            return;
        }

        let is_playing = player.playback_state == PlaybackState::Playing;

        // Calculate audio energy from low-frequency spectrum and RMS
        let bass_energy: f32 = if !audio_frame.spectrum.is_empty() {
            audio_frame.spectrum.iter().take(8).sum::<f32>() / 8.0
        } else {
            0.0
        };
        let avg_rms = (audio_frame.rms_left + audio_frame.rms_right) * 0.5;
        let energy = (bass_energy * 0.7 + avg_rms * 0.3).clamp(0.0, 1.0);

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);

        let (pet_lines, note_line) = if is_playing {
            // Speed up bounce animation frame rate when bass / energy spikes
            let step_duration = if energy > 0.45 {
                180
            } else if energy > 0.2 {
                320
            } else {
                500
            };
            let frame_idx = ((now_ms / step_duration) % 4) as usize;
            (
                Self::playing_frame(frame_idx, theme),
                Self::playing_notes(energy, theme),
            )
        } else {
            let sleep_idx = ((now_ms / 800) % 2) as usize;
            (
                Self::sleeping_frame(sleep_idx, theme),
                Self::sleeping_notes(theme),
            )
        };

        // Center vertically inside box
        let total_lines = pet_lines.len() + 1; // Pet lines + note line
        let top_padding = (inner.height.saturating_sub(total_lines as u16)) / 2;

        let mut rendered_lines = Vec::with_capacity(top_padding as usize + total_lines);
        for _ in 0..top_padding {
            rendered_lines.push(Line::from(""));
        }

        rendered_lines.extend(pet_lines);
        rendered_lines.push(note_line);

        frame.render_widget(
            Paragraph::new(rendered_lines).alignment(Alignment::Center),
            inner,
        );
    }

    /// 4 ANSI-styled dancing frames with DJ headphones, blush cheeks, and distinct fur/accessory colors
    fn playing_frame(idx: usize, theme: &Theme) -> Vec<Line<'static>> {
        let fur = Style::default().fg(theme.text).add_modifier(Modifier::BOLD);
        let ear_pink = Style::default().fg(Color::Rgb(251, 146, 160));
        let blush = Style::default().fg(Color::Rgb(255, 128, 160));
        let phones = Style::default()
            .fg(theme.visualizer_secondary)
            .add_modifier(Modifier::BOLD);
        let eye = Style::default().fg(theme.text).add_modifier(Modifier::BOLD);
        let nose = Style::default().fg(Color::Rgb(251, 146, 160));
        let note_accent = Style::default().fg(theme.accent).add_modifier(Modifier::BOLD);
        let note_sec = Style::default()
            .fg(theme.visualizer_secondary)
            .add_modifier(Modifier::BOLD);

        match idx % 4 {
            // Frame 0: Groove Left
            0 => vec![
                Line::from(vec![
                    Span::styled("[ ", phones),
                    Span::styled("/", fur),
                    Span::styled("\\", ear_pink),
                    Span::styled("_", fur),
                    Span::styled("/", ear_pink),
                    Span::styled("\\", fur),
                    Span::styled(" ]", phones),
                ]),
                Line::from(vec![
                    Span::styled(" (", fur),
                    Span::styled("•", blush),
                    Span::styled("^", eye),
                    Span::styled(".", nose),
                    Span::styled("^", eye),
                    Span::styled("•", blush),
                    Span::styled(")ﾉ", fur),
                ]),
                Line::from(vec![
                    Span::styled("  / >", fur),
                    Span::styled("♫", note_accent),
                    Span::styled("  ", fur),
                ]),
            ],
            // Frame 1: Jump / Hands up
            1 => vec![
                Line::from(vec![
                    Span::styled("[ ", phones),
                    Span::styled("/", fur),
                    Span::styled("\\", ear_pink),
                    Span::styled("_", fur),
                    Span::styled("/", ear_pink),
                    Span::styled("\\", fur),
                    Span::styled(" ]", phones),
                ]),
                Line::from(vec![
                    Span::styled("\\(", fur),
                    Span::styled("•", blush),
                    Span::styled("o", eye),
                    Span::styled(".", nose),
                    Span::styled("o", eye),
                    Span::styled("•", blush),
                    Span::styled(")/", fur),
                ]),
                Line::from(vec![
                    Span::styled("  | | ", fur),
                    Span::styled("♫", note_sec),
                    Span::styled(" ", fur),
                ]),
            ],
            // Frame 2: Groove Right / Wink
            2 => vec![
                Line::from(vec![
                    Span::styled("[ ", phones),
                    Span::styled("/", fur),
                    Span::styled("\\", ear_pink),
                    Span::styled("_", fur),
                    Span::styled("/", ear_pink),
                    Span::styled("\\", fur),
                    Span::styled(" ]", phones),
                ]),
                Line::from(vec![
                    Span::styled("ヽ(", fur),
                    Span::styled("•", blush),
                    Span::styled("^", eye),
                    Span::styled(".", nose),
                    Span::styled("~", eye),
                    Span::styled("•", blush),
                    Span::styled(") ", fur),
                ]),
                Line::from(vec![
                    Span::styled("  ", fur),
                    Span::styled("♫", note_accent),
                    Span::styled("< \\  ", fur),
                ]),
            ],
            // Frame 3: Bass Drop / Jamming
            _ => vec![
                Line::from(vec![
                    Span::styled("[ ", phones),
                    Span::styled("/", fur),
                    Span::styled("\\", ear_pink),
                    Span::styled("_", fur),
                    Span::styled("/", ear_pink),
                    Span::styled("\\", fur),
                    Span::styled(" ]", phones),
                ]),
                Line::from(vec![
                    Span::styled(" (", fur),
                    Span::styled("•", blush),
                    Span::styled(">", eye),
                    Span::styled(".", nose),
                    Span::styled("<", eye),
                    Span::styled("•", blush),
                    Span::styled(") ", fur),
                ]),
                Line::from(vec![
                    Span::styled(" ( v )", fur),
                    Span::styled("♫", note_accent),
                    Span::styled(" ", fur),
                ]),
            ],
        }
    }

    /// Audio-reactive floating notes when playing
    fn playing_notes(energy: f32, theme: &Theme) -> Line<'static> {
        let note_accent = Style::default().fg(theme.accent).add_modifier(Modifier::BOLD);
        let note_sec = Style::default()
            .fg(theme.visualizer_secondary)
            .add_modifier(Modifier::BOLD);

        if energy > 0.5 {
            Line::from(vec![
                Span::styled("♫ ", note_accent),
                Span::styled("♪ ", note_sec),
                Span::styled("♫ ", note_accent),
                Span::styled("♪", note_sec),
            ])
        } else if energy > 0.2 {
            Line::from(vec![
                Span::styled("♪ ", note_sec),
                Span::styled("♫ ", note_accent),
                Span::styled("♪", note_sec),
            ])
        } else {
            Line::from(vec![Span::styled(
                "  ♪  ",
                Style::default().fg(theme.visualizer_primary),
            )])
        }
    }

    /// 2 ANSI-styled restful sleep frames with closed eyes and drifting z's
    fn sleeping_frame(idx: usize, theme: &Theme) -> Vec<Line<'static>> {
        let fur_dim = Style::default().fg(theme.text_dim);
        let ear_pink = Style::default().fg(Color::Rgb(251, 146, 160));
        let nose = Style::default().fg(Color::Rgb(251, 146, 160));
        let eye = Style::default().fg(theme.text_muted);
        let z_dim = Style::default().fg(theme.text_dim);
        let z_bright = Style::default()
            .fg(theme.text_muted)
            .add_modifier(Modifier::BOLD);

        if idx.is_multiple_of(2) {
            vec![
                Line::from(vec![
                    Span::styled(" /", fur_dim),
                    Span::styled("\\", ear_pink),
                    Span::styled("_", fur_dim),
                    Span::styled("/", ear_pink),
                    Span::styled("\\", fur_dim),
                    Span::styled("   ", fur_dim),
                    Span::styled("z", z_dim),
                ]),
                Line::from(vec![
                    Span::styled(" ( ", fur_dim),
                    Span::styled("-", eye),
                    Span::styled(".", nose),
                    Span::styled("-", eye),
                    Span::styled(" )  ", fur_dim),
                    Span::styled("Z", z_bright),
                ]),
                Line::from(vec![Span::styled("  (   )   ", fur_dim)]),
            ]
        } else {
            vec![
                Line::from(vec![
                    Span::styled(" /", fur_dim),
                    Span::styled("\\", ear_pink),
                    Span::styled("_", fur_dim),
                    Span::styled("/", ear_pink),
                    Span::styled("\\", fur_dim),
                    Span::styled("   ", fur_dim),
                    Span::styled("Z", z_bright),
                ]),
                Line::from(vec![
                    Span::styled(" ( ", fur_dim),
                    Span::styled("-", eye),
                    Span::styled(".", nose),
                    Span::styled("-", eye),
                    Span::styled(" )  ", fur_dim),
                    Span::styled("z", z_dim),
                ]),
                Line::from(vec![Span::styled("  (   )   ", fur_dim)]),
            ]
        }
    }

    /// Floating sleep indicator when stopped/paused
    fn sleeping_notes(theme: &Theme) -> Line<'static> {
        Line::from(vec![Span::styled(
            "zzZ",
            Style::default().fg(theme.text_dim),
        )])
    }
}

