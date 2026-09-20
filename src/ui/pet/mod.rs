mod sprites;

use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::audio::{AudioFrame, PlaybackState};
use crate::player::PlayerState;
use crate::theme::Theme;
use crate::ui::player::{HitAction, HitZone};

/// Smallest inner pane that fits each sprite tier. Each height leaves one
/// spare row so the note line under the cat stays visible.
const MEDIUM_MIN_WIDTH: u16 = 12;
const MEDIUM_MIN_HEIGHT: u16 = 7;
const LARGE_MIN_WIDTH: u16 = 16;
const LARGE_MIN_HEIGHT: u16 = 9;

/// How long the cat stays happy after a pet, in milliseconds
pub const PET_REACTION_MS: u128 = 1_200;

/// Which sprite tier the current pane can hold
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum PetSize {
    Small,
    Medium,
    Large,
}

impl PetSize {
    fn for_pane(width: u16, height: u16) -> Self {
        if width >= LARGE_MIN_WIDTH && height >= LARGE_MIN_HEIGHT {
            Self::Large
        } else if width >= MEDIUM_MIN_WIDTH && height >= MEDIUM_MIN_HEIGHT {
            Self::Medium
        } else {
            Self::Small
        }
    }
}

/// Palette token for ANSI half-block pixel art
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum SpriteColor {
    Trans,
    Fur,
    FurDim,
    EarPink,
    Blush,
    Nose,
    Muzzle,
    Headband,
    HeadphoneCup,
    HeadphoneGlow,
    EyeDark,
    EyeWhite,
    Collar,
}

pub struct PetView;

impl PetView {
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        player: &PlayerState,
        audio_frame: &AudioFrame,
        theme: &Theme,
        reaction: Option<f32>,
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

        // Clicking or tapping the pet pane pets the cat. Hiding the pane lives
        // on the header badge and the `x` key instead.
        hit_zones.push(HitZone {
            rect: area,
            action: HitAction::PetInteract,
        });

        if inner.height < 4 || inner.width < 10 {
            return;
        }

        let size = PetSize::for_pane(inner.width, inner.height);
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

        let (pet_lines, note_line) = if let Some(progress) = reaction {
            // A pet wakes the cat up no matter what playback is doing, so it
            // bounces on the awake frames for the length of the reaction.
            let frame_idx = ((now_ms / 150) % 4) as usize;
            (
                Self::happy_frame(frame_idx, theme, size),
                Self::reaction_hearts(progress, theme),
            )
        } else if is_playing {
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
                Self::playing_frame(frame_idx, theme, size),
                Self::playing_notes(energy, theme),
            )
        } else {
            let sleep_idx = ((now_ms / 800) % 2) as usize;
            (
                Self::sleeping_frame(sleep_idx, theme, size),
                Self::sleeping_notes(theme),
            )
        };

        let show_notes = inner.height as usize > pet_lines.len();
        let total_lines = pet_lines.len() + if show_notes { 1 } else { 0 };
        let top_padding = inner.height.saturating_sub(total_lines as u16) / 2;

        let mut rendered_lines = Vec::with_capacity(top_padding as usize + total_lines);
        for _ in 0..top_padding {
            rendered_lines.push(Line::from(""));
        }

        rendered_lines.extend(pet_lines);
        if show_notes {
            rendered_lines.push(note_line);
        }

        frame.render_widget(
            Paragraph::new(rendered_lines).alignment(Alignment::Center),
            inner,
        );
    }

    /// Convert pixel character to color token
    fn char_to_token(b: u8) -> SpriteColor {
        match b {
            b'F' => SpriteColor::Fur,
            b'f' => SpriteColor::FurDim,
            b'P' => SpriteColor::EarPink,
            b'B' => SpriteColor::Blush,
            b'N' => SpriteColor::Nose,
            b'M' => SpriteColor::Muzzle,
            b'H' => SpriteColor::Headband,
            b'C' => SpriteColor::HeadphoneCup,
            b'c' => SpriteColor::HeadphoneGlow,
            b'E' => SpriteColor::EyeDark,
            b'W' => SpriteColor::EyeWhite,
            b'A' => SpriteColor::Collar,
            _ => SpriteColor::Trans,
        }
    }

    /// Resolve color token to concrete theme color
    fn resolve_color(
        token: SpriteColor,
        theme: &Theme,
        is_sleeping: bool,
        is_happy: bool,
    ) -> Option<Color> {
        match token {
            SpriteColor::Trans => None,
            SpriteColor::Fur => Some(if is_sleeping { theme.text_dim } else { theme.text }),
            SpriteColor::FurDim => Some(theme.text_muted),
            SpriteColor::EarPink => Some(if is_sleeping {
                Color::Rgb(200, 120, 140)
            } else if is_happy {
                Color::Rgb(255, 176, 190)
            } else {
                Color::Rgb(251, 146, 160)
            }),
            SpriteColor::Blush => Some(if is_happy {
                Color::Rgb(255, 150, 185)
            } else {
                Color::Rgb(255, 110, 150)
            }),
            SpriteColor::Nose => Some(Color::Rgb(251, 146, 160)),
            SpriteColor::Muzzle => Some(if is_sleeping {
                Color::Rgb(180, 168, 172)
            } else {
                Color::Rgb(255, 232, 238)
            }),
            SpriteColor::Headband => Some(if is_sleeping {
                theme.text_dim
            } else {
                theme.visualizer_primary
            }),
            SpriteColor::HeadphoneCup => Some(if is_sleeping {
                theme.text_dim
            } else {
                theme.visualizer_secondary
            }),
            SpriteColor::HeadphoneGlow => Some(theme.accent),
            SpriteColor::EyeDark => Some(Color::Rgb(22, 22, 28)),
            SpriteColor::EyeWhite => Some(Color::Rgb(255, 255, 255)),
            SpriteColor::Collar => Some(if is_sleeping {
                theme.text_dim
            } else {
                theme.accent
            }),
        }
    }

    /// Convert two vertical pixels into an ANSI half-block span
    fn pixel_pair_to_span(
        top: Option<Color>,
        bottom: Option<Color>,
        bg_color: Color,
    ) -> Span<'static> {
        match (top, bottom) {
            (None, None) => Span::raw(" "),
            (Some(tc), None) => Span::styled("▀", Style::default().fg(tc).bg(bg_color)),
            (None, Some(bc)) => Span::styled("▄", Style::default().fg(bc).bg(bg_color)),
            (Some(tc), Some(bc)) => {
                if tc == bc {
                    Span::styled("█", Style::default().fg(tc))
                } else {
                    Span::styled("▀", Style::default().fg(tc).bg(bc))
                }
            }
        }
    }

    /// Render a pixel art matrix into terminal rows using ANSI half-blocks.
    /// Two pixel rows collapse into one terminal row, so the grid needs an even
    /// row count.
    fn render_pixel_grid(
        grid: &[&str],
        theme: &Theme,
        is_sleeping: bool,
        is_happy: bool,
    ) -> Vec<Line<'static>> {
        let rows = grid.len() / 2;
        let mut lines = Vec::with_capacity(rows);

        for row in 0..rows {
            let top_row = grid[row * 2].as_bytes();
            let bot_row = grid[row * 2 + 1].as_bytes();
            let width = top_row.len().min(bot_row.len());

            let mut spans = Vec::with_capacity(width);
            for col in 0..width {
                let top_token = Self::char_to_token(top_row[col]);
                let bot_token = Self::char_to_token(bot_row[col]);

                let top_color = Self::resolve_color(top_token, theme, is_sleeping, is_happy);
                let bot_color = Self::resolve_color(bot_token, theme, is_sleeping, is_happy);

                spans.push(Self::pixel_pair_to_span(top_color, bot_color, theme.bg));
            }
            lines.push(Line::from(spans));
        }

        lines
    }

    /// Pick one of the four dancing frames at the requested detail level
    fn playing_frame(idx: usize, theme: &Theme, size: PetSize) -> Vec<Line<'static>> {
        let idx = idx % 4;
        match size {
            PetSize::Large => {
                Self::render_pixel_grid(&sprites::LARGE_PLAYING[idx], theme, false, false)
            }
            PetSize::Medium => {
                Self::render_pixel_grid(&sprites::MEDIUM_PLAYING[idx], theme, false, false)
            }
            PetSize::Small => {
                Self::render_pixel_grid(&sprites::SMALL_PLAYING[idx], theme, false, false)
            }
        }
    }

    /// Audio-reactive floating notes when playing
    fn playing_notes(energy: f32, theme: &Theme) -> Line<'static> {
        let note_accent = Style::default().fg(theme.accent).add_modifier(Modifier::BOLD);
        let note_sec = Style::default()
            .fg(theme.visualizer_secondary)
            .add_modifier(Modifier::BOLD);
        let note_prim = Style::default().fg(theme.visualizer_primary);

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
                Span::styled("♪", note_prim),
            ])
        } else {
            Line::from(vec![Span::styled("♪", note_prim)])
        }
    }

    /// Dancing frames tinted brighter while the cat is being petted
    fn happy_frame(idx: usize, theme: &Theme, size: PetSize) -> Vec<Line<'static>> {
        let idx = idx % 4;
        match size {
            PetSize::Large => {
                Self::render_pixel_grid(&sprites::LARGE_PLAYING[idx], theme, false, true)
            }
            PetSize::Medium => {
                Self::render_pixel_grid(&sprites::MEDIUM_PLAYING[idx], theme, false, true)
            }
            PetSize::Small => {
                Self::render_pixel_grid(&sprites::SMALL_PLAYING[idx], theme, false, true)
            }
        }
    }

    /// Hearts that pop out of the cat and fade over the reaction window.
    /// `progress` runs 0.0 at the moment of the pet to 1.0 when it wears off.
    fn reaction_hearts(progress: f32, theme: &Theme) -> Line<'static> {
        let progress = progress.clamp(0.0, 1.0);

        // Fade the hearts from blush pink toward the muted text color so they
        // dissolve instead of blinking out.
        let color = if progress < 0.55 {
            Color::Rgb(255, 130, 170)
        } else if progress < 0.8 {
            theme.accent
        } else {
            theme.text_muted
        };
        let style = Style::default().fg(color).add_modifier(Modifier::BOLD);

        // One heart at the pop, spreading to three as it rises
        let glyphs = if progress < 0.25 {
            "\u{2665}"
        } else if progress < 0.6 {
            "\u{2661} \u{2665} \u{2661}"
        } else {
            "\u{2665} \u{2661} \u{2665}"
        };

        Line::from(Span::styled(glyphs, style))
    }

    /// Pick one of the two breathing sleep frames at the requested detail level
    fn sleeping_frame(idx: usize, theme: &Theme, size: PetSize) -> Vec<Line<'static>> {
        let idx = idx % 2;
        match size {
            PetSize::Large => {
                Self::render_pixel_grid(&sprites::LARGE_SLEEPING[idx], theme, true, false)
            }
            PetSize::Medium => {
                Self::render_pixel_grid(&sprites::MEDIUM_SLEEPING[idx], theme, true, false)
            }
            PetSize::Small => {
                Self::render_pixel_grid(&sprites::SMALL_SLEEPING[idx], theme, true, false)
            }
        }
    }

    /// Floating sleep indicator when stopped/paused
    fn sleeping_notes(theme: &Theme) -> Line<'static> {
        Line::from(vec![
            Span::styled("z", Style::default().fg(theme.text_dim)),
            Span::styled("z", Style::default().fg(theme.text_muted)),
            Span::styled(
                "Z",
                Style::default()
                    .fg(theme.text_muted)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::sprites::{
        LARGE_PLAYING, LARGE_SLEEPING, MEDIUM_PLAYING, MEDIUM_SLEEPING, SMALL_PLAYING,
        SMALL_SLEEPING,
    };
    use super::{PetSize, PetView, SpriteColor, LARGE_MIN_HEIGHT, LARGE_MIN_WIDTH,
        MEDIUM_MIN_HEIGHT, MEDIUM_MIN_WIDTH};

    /// Every row in a grid must be the same width, or half-block pairing
    /// silently truncates the sprite to the shortest row.
    fn assert_rectangular(grid: &[&str], expected_width: usize, label: &str) {
        for (i, row) in grid.iter().enumerate() {
            assert_eq!(
                row.len(),
                expected_width,
                "{label} row {i} is {} wide, expected {expected_width}",
                row.len()
            );
        }
    }

    /// Unknown palette characters render as transparent holes, so catch typos.
    fn assert_known_tokens(grid: &[&str], label: &str) {
        for (i, row) in grid.iter().enumerate() {
            for (c, b) in row.bytes().enumerate() {
                if b == b'.' {
                    continue;
                }
                assert_ne!(
                    PetView::char_to_token(b),
                    SpriteColor::Trans,
                    "{label} row {i} col {c} uses unknown palette char '{}'",
                    b as char
                );
            }
        }
    }

    fn check_tier(playing: &[[&str; 16]], sleeping: &[[&str; 16]], width: usize, label: &str) {
        for (i, grid) in playing.iter().enumerate() {
            assert_rectangular(grid, width, &format!("{label}_PLAYING[{i}]"));
            assert_known_tokens(grid, &format!("{label}_PLAYING[{i}]"));
        }
        for (i, grid) in sleeping.iter().enumerate() {
            assert_rectangular(grid, width, &format!("{label}_SLEEPING[{i}]"));
            assert_known_tokens(grid, &format!("{label}_SLEEPING[{i}]"));
        }
    }

    #[test]
    fn large_frames_are_16x16() {
        check_tier(&LARGE_PLAYING, &LARGE_SLEEPING, 16, "LARGE");
    }

    #[test]
    fn medium_frames_are_12x12() {
        for (i, grid) in MEDIUM_PLAYING.iter().enumerate() {
            assert_rectangular(grid, 12, &format!("MEDIUM_PLAYING[{i}]"));
            assert_known_tokens(grid, &format!("MEDIUM_PLAYING[{i}]"));
        }
        for (i, grid) in MEDIUM_SLEEPING.iter().enumerate() {
            assert_rectangular(grid, 12, &format!("MEDIUM_SLEEPING[{i}]"));
            assert_known_tokens(grid, &format!("MEDIUM_SLEEPING[{i}]"));
        }
    }

    #[test]
    fn small_frames_are_10x8() {
        for (i, grid) in SMALL_PLAYING.iter().enumerate() {
            assert_rectangular(grid, 10, &format!("SMALL_PLAYING[{i}]"));
            assert_known_tokens(grid, &format!("SMALL_PLAYING[{i}]"));
        }
        for (i, grid) in SMALL_SLEEPING.iter().enumerate() {
            assert_rectangular(grid, 10, &format!("SMALL_SLEEPING[{i}]"));
            assert_known_tokens(grid, &format!("SMALL_SLEEPING[{i}]"));
        }
    }

    /// Each sprite must fit the smallest pane that selects it, with a spare
    /// row left for the note line underneath.
    #[test]
    fn every_tier_fits_the_pane_that_selects_it() {
        let large_rows = (LARGE_PLAYING[0].len() / 2) as u16;
        assert!(large_rows < LARGE_MIN_HEIGHT, "large sprite crowds out the note line");
        assert!(LARGE_PLAYING[0][0].len() as u16 <= LARGE_MIN_WIDTH);

        let medium_rows = (MEDIUM_PLAYING[0].len() / 2) as u16;
        assert!(medium_rows < MEDIUM_MIN_HEIGHT, "medium sprite crowds out the note line");
        assert!(MEDIUM_PLAYING[0][0].len() as u16 <= MEDIUM_MIN_WIDTH);
    }

    #[test]
    fn pane_size_picks_the_right_tier() {
        assert_eq!(PetSize::for_pane(22, 11), PetSize::Large);
        assert_eq!(PetSize::for_pane(16, 9), PetSize::Large);
        // One row short of large falls back rather than clipping
        assert_eq!(PetSize::for_pane(16, 8), PetSize::Medium);
        assert_eq!(PetSize::for_pane(14, 7), PetSize::Medium);
        assert_eq!(PetSize::for_pane(12, 6), PetSize::Small);
        assert_eq!(PetSize::for_pane(10, 4), PetSize::Small);
    }
}
