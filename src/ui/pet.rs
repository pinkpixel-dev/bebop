use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
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
            .title(Span::styled(" 🐾 Pet ", Style::default().fg(theme.text).add_modifier(Modifier::BOLD)))
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

        let (pet_lines, note_str, note_style) = if is_playing {
            // Speed up bounce animation frame rate when bass / energy spikes
            let step_duration = if energy > 0.45 {
                180
            } else if energy > 0.2 {
                320
            } else {
                500
            };
            let frame_idx = ((now_ms / step_duration) % 4) as usize;

            let frames: [[&str; 3]; 4] = [
                [" /\\_/\\  ", "( ^.^ )ﾉ", " / >♫  "],
                [" /\\_/\\  ", "\\(^o^)/ ", "  | | ♫ "],
                [" /\\_/\\  ", "ヽ( ^.^) ", "  ♫< \\  "],
                [" /\\_/\\  ", "( >.< ) ", " ( v )♫ "],
            ];

            let notes = if energy > 0.5 {
                "♫ ♪ ♫ ♪"
            } else if energy > 0.2 {
                "♪ ♫ ♪"
            } else {
                "  ♪  "
            };

            let n_style = if energy > 0.4 {
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.visualizer_primary)
            };

            (frames[frame_idx], notes, n_style)
        } else {
            let sleep_frames: [[&str; 3]; 2] = [
                [" /\\_/\\ z", "( -.- )Z", " (   )  "],
                [" /\\_/\\ Z", "( -.- )z", " (   )  "],
            ];
            let sleep_idx = ((now_ms / 800) % 2) as usize;
            (sleep_frames[sleep_idx], "zzZ", Style::default().fg(theme.text_dim))
        };

        // Center vertically inside box
        let total_lines = 4; // 3 pet lines + 1 note line
        let top_padding = (inner.height.saturating_sub(total_lines)) / 2;

        let mut rendered_lines = Vec::new();
        for _ in 0..top_padding {
            rendered_lines.push(Line::from(""));
        }

        for line in pet_lines {
            rendered_lines.push(Line::from(Span::styled(
                line,
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            )));
        }

        rendered_lines.push(Line::from(Span::styled(note_str, note_style)));

        frame.render_widget(
            Paragraph::new(rendered_lines).alignment(Alignment::Center),
            inner,
        );
    }
}
