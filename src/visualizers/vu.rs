use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::Frame;

use crate::audio::frame::AudioFrame;
use crate::theme::Theme;
use crate::visualizers::Visualizer;

const H_BLOCKS: [char; 9] = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];

/// Stereo VU Meter visualizer with Left/Right channel separation,
/// peak hold decay, 0 dBFS calibration, and clipping indicators.
pub struct VuMeterVisualizer {
    peak_hold_left: f32,
    peak_hold_right: f32,
    peak_decay_timer_left: usize,
    peak_decay_timer_right: usize,
    clip_left_timer: usize,
    clip_right_timer: usize,
}

impl Default for VuMeterVisualizer {
    fn default() -> Self {
        Self {
            peak_hold_left: 0.0,
            peak_hold_right: 0.0,
            peak_decay_timer_left: 0,
            peak_decay_timer_right: 0,
            clip_left_timer: 0,
            clip_right_timer: 0,
        }
    }
}

impl VuMeterVisualizer {
    fn to_db(amp: f32) -> f32 {
        if amp > 0.0001 {
            20.0 * amp.log10()
        } else {
            -60.0
        }
    }

    /// Maps decibels (-48 dB to +3 dB) to a normalized 0.0..1.0 range.
    /// 0 dB sits at 48.0 / 51.0 = ~0.941.
    fn db_to_ratio(db: f32) -> f32 {
        ((db + 48.0) / 51.0).clamp(0.0, 1.0)
    }

    fn ratio_to_db(ratio: f32) -> f32 {
        ratio * 51.0 - 48.0
    }
}

impl Visualizer for VuMeterVisualizer {
    fn name(&self) -> &'static str {
        "Stereo VU Meter"
    }

    fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        audio: &AudioFrame,
        theme: &Theme,
    ) {
        if area.width < 18 || area.height < 2 {
            return;
        }

        let width = area.width as usize;
        let height = area.height as usize;
        let buf = frame.buffer_mut();

        // Update peak hold with decay physics
        if audio.peak_left >= self.peak_hold_left {
            self.peak_hold_left = audio.peak_left;
            self.peak_decay_timer_left = 15;
        } else if self.peak_decay_timer_left > 0 {
            self.peak_decay_timer_left -= 1;
        } else {
            self.peak_hold_left = (self.peak_hold_left - 0.015).max(0.0);
        }

        if audio.peak_right >= self.peak_hold_right {
            self.peak_hold_right = audio.peak_right;
            self.peak_decay_timer_right = 15;
        } else if self.peak_decay_timer_right > 0 {
            self.peak_decay_timer_right -= 1;
        } else {
            self.peak_hold_right = (self.peak_hold_right - 0.015).max(0.0);
        }

        // Clipping indicator trigger (held for 20 frames)
        if audio.peak_left >= 0.99 {
            self.clip_left_timer = 20;
        } else if self.clip_left_timer > 0 {
            self.clip_left_timer -= 1;
        }

        if audio.peak_right >= 0.99 {
            self.clip_right_timer = 20;
        } else if self.clip_right_timer > 0 {
            self.clip_right_timer -= 1;
        }

        // Meter bar horizontal bounds
        let label_width = 3usize; // "L  " or "R  "
        let readout_width = 11usize; // " -12.4 dB  " or "   CLIP    "
        let bar_start_x = area.left() + label_width as u16;
        let bar_width = width.saturating_sub(label_width + readout_width).max(4);

        let zero_db_col = (Self::db_to_ratio(0.0) * (bar_width - 1) as f32).round() as usize;

        // Render layout according to available height
        if height >= 8 {
            // High-resolution studio layout
            let scale_y = area.top() + 1;
            let l_bar_y1 = area.top() + 2;
            let l_bar_y2 = area.top() + 3;
            let r_bar_y1 = area.top() + 5;
            let r_bar_y2 = area.top() + 6;
            let stats_y = area.top() + 7;

            // dB scale markings
            self.render_db_scale(buf, bar_start_x, scale_y, bar_width, theme);

            // Left channel rows (2 rows thick)
            self.render_channel(
                buf,
                area.left(),
                l_bar_y1,
                "L",
                audio.rms_left,
                self.peak_hold_left,
                self.clip_left_timer > 0,
                bar_start_x,
                bar_width,
                zero_db_col,
                theme,
                true,
            );
            self.render_channel_bar_only(
                buf,
                l_bar_y2,
                audio.rms_left,
                self.peak_hold_left,
                bar_start_x,
                bar_width,
                zero_db_col,
                theme,
            );

            // Right channel rows (2 rows thick)
            self.render_channel(
                buf,
                area.left(),
                r_bar_y1,
                "R",
                audio.rms_right,
                self.peak_hold_right,
                self.clip_right_timer > 0,
                bar_start_x,
                bar_width,
                zero_db_col,
                theme,
                true,
            );
            self.render_channel_bar_only(
                buf,
                r_bar_y2,
                audio.rms_right,
                self.peak_hold_right,
                bar_start_x,
                bar_width,
                zero_db_col,
                theme,
            );

            // Bottom stereo balance & headroom stats
            if stats_y < area.bottom() {
                self.render_stats(buf, area.left(), stats_y, width, audio, theme);
            }
        } else if height >= 4 {
            // Medium compact layout
            let scale_y = area.top();
            let l_bar_y = area.top() + 1;
            let r_bar_y = area.top() + 2;

            self.render_db_scale(buf, bar_start_x, scale_y, bar_width, theme);

            self.render_channel(
                buf,
                area.left(),
                l_bar_y,
                "L",
                audio.rms_left,
                self.peak_hold_left,
                self.clip_left_timer > 0,
                bar_start_x,
                bar_width,
                zero_db_col,
                theme,
                true,
            );

            self.render_channel(
                buf,
                area.left(),
                r_bar_y,
                "R",
                audio.rms_right,
                self.peak_hold_right,
                self.clip_right_timer > 0,
                bar_start_x,
                bar_width,
                zero_db_col,
                theme,
                true,
            );

            if height >= 5 {
                let stats_y = area.top() + 3;
                self.render_stats(buf, area.left(), stats_y, width, audio, theme);
            }
        } else {
            // Ultra-compact 2-row layout
            let l_bar_y = area.top();
            let r_bar_y = area.top() + 1;

            self.render_channel(
                buf,
                area.left(),
                l_bar_y,
                "L",
                audio.rms_left,
                self.peak_hold_left,
                self.clip_left_timer > 0,
                bar_start_x,
                bar_width,
                zero_db_col,
                theme,
                false,
            );

            self.render_channel(
                buf,
                area.left(),
                r_bar_y,
                "R",
                audio.rms_right,
                self.peak_hold_right,
                self.clip_right_timer > 0,
                bar_start_x,
                bar_width,
                zero_db_col,
                theme,
                false,
            );
        }
    }
}

impl VuMeterVisualizer {
    /// Renders calibrated dB scale tick marks above the meter bars.
    fn render_db_scale(
        &self,
        buf: &mut ratatui::buffer::Buffer,
        start_x: u16,
        y: u16,
        bar_width: usize,
        theme: &Theme,
    ) {
        let marks: &[(f32, &str)] = if bar_width >= 35 {
            &[(-40.0, "-40"), (-24.0, "-24"), (-12.0, "-12"), (-6.0, "-6"), (-3.0, "-3"), (0.0, "0"), (3.0, "+3")]
        } else if bar_width >= 20 {
            &[(-30.0, "-30"), (-12.0, "-12"), (-6.0, "-6"), (0.0, "0")]
        } else {
            &[(-20.0, "-20"), (0.0, "0")]
        };

        for &(db, label) in marks {
            let ratio = Self::db_to_ratio(db);
            let col = (ratio * (bar_width - 1) as f32).round() as u16;
            let target_x = start_x + col;

            // Offset label so it is roughly centered over the tick
            let label_len = label.len() as u16;
            let print_x = target_x.saturating_sub(label_len / 2);

            let style = if db >= 0.0 {
                Style::default().fg(Color::Rgb(244, 63, 94)).add_modifier(Modifier::BOLD)
            } else if db >= -6.0 {
                Style::default().fg(theme.visualizer_primary)
            } else {
                Style::default().fg(theme.text_dim)
            };

            for (i, ch) in label.chars().enumerate() {
                let cell_x = print_x + i as u16;
                if cell_x >= start_x && cell_x < start_x + bar_width as u16 {
                    if let Some(cell) = buf.cell_mut((cell_x, y)) {
                        cell.set_char(ch);
                        cell.set_style(style);
                    }
                }
            }
        }
    }

    /// Renders a single channel meter row: Label + Bar + Decibel/Clip readout.
    fn render_channel(
        &self,
        buf: &mut ratatui::buffer::Buffer,
        start_x: u16,
        y: u16,
        label: &str,
        rms: f32,
        peak: f32,
        is_clipping: bool,
        bar_start_x: u16,
        bar_width: usize,
        zero_db_col: usize,
        theme: &Theme,
        include_readout: bool,
    ) {
        // 1. Channel label ("L " / "R ")
        if let Some(cell) = buf.cell_mut((start_x, y)) {
            cell.set_char(label.chars().next().unwrap_or(' '));
            cell.set_style(Style::default().fg(theme.text).add_modifier(Modifier::BOLD));
        }

        // 2. Bar track
        self.render_channel_bar_only(buf, y, rms, peak, bar_start_x, bar_width, zero_db_col, theme);

        // 3. Decibel / Clipping readout on the right
        if include_readout {
            let readout_x = bar_start_x + bar_width as u16 + 1;
            let (text, style) = if is_clipping {
                ("  CLIP  ".to_string(), Style::default().fg(Color::Rgb(244, 63, 94)).add_modifier(Modifier::BOLD))
            } else {
                let db = Self::to_db(rms);
                let formatted = if db <= -48.0 {
                    " -inf dB".to_string()
                } else {
                    format!("{:>5.1} dB", db)
                };
                let col = if db >= 0.0 {
                    Color::Rgb(244, 63, 94)
                } else if db >= -6.0 {
                    theme.visualizer_primary
                } else {
                    theme.text_muted
                };
                (formatted, Style::default().fg(col))
            };

            for (i, ch) in text.chars().enumerate() {
                let cell_x = readout_x + i as u16;
                if let Some(cell) = buf.cell_mut((cell_x, y)) {
                    cell.set_char(ch);
                    cell.set_style(style);
                }
            }
        }
    }

    /// Draws only the meter track cells (for multi-row meter thickness).
    fn render_channel_bar_only(
        &self,
        buf: &mut ratatui::buffer::Buffer,
        y: u16,
        rms: f32,
        peak: f32,
        bar_start_x: u16,
        bar_width: usize,
        zero_db_col: usize,
        theme: &Theme,
    ) {
        let rms_db = Self::to_db(rms);
        let rms_ratio = Self::db_to_ratio(rms_db);
        let peak_db = Self::to_db(peak);
        let peak_ratio = Self::db_to_ratio(peak_db);

        let total_sub_steps = bar_width * 8;
        let filled_sub_steps = (rms_ratio * total_sub_steps as f32).round() as usize;
        let peak_col = (peak_ratio * (bar_width - 1) as f32).round() as usize;

        for col in 0..bar_width {
            let cell_x = bar_start_x + col as u16;
            let cell = match buf.cell_mut((cell_x, y)) {
                Some(c) => c,
                None => continue,
            };

            let cell_ratio = col as f32 / bar_width.max(1) as f32;
            let cell_db = Self::ratio_to_db(cell_ratio);

            let cell_color = if cell_db >= 0.0 {
                Color::Rgb(244, 63, 94) // Red for clip / 0 dB+
            } else if cell_db >= -6.0 {
                theme.visualizer_primary // Warm accent for nominal peak range
            } else {
                theme.visualizer_secondary // Cool tone for normal operating level
            };

            let col_step_start = col * 8;
            if filled_sub_steps >= (col + 1) * 8 {
                // Fully filled block
                cell.set_char(H_BLOCKS[8]);
                cell.set_style(Style::default().fg(cell_color));
            } else if filled_sub_steps > col_step_start {
                // Partial block
                let sub = filled_sub_steps - col_step_start;
                cell.set_char(H_BLOCKS[sub.min(8)]);
                cell.set_style(Style::default().fg(cell_color));
            } else if col == peak_col && peak_db > -45.0 {
                // Peak hold indicator
                cell.set_char('▌');
                cell.set_style(Style::default().fg(theme.visualizer_peak).add_modifier(Modifier::BOLD));
            } else if col == zero_db_col {
                // 0 dB calibration tick
                cell.set_char('┊');
                cell.set_style(Style::default().fg(theme.border_focused));
            } else {
                // Empty groove
                cell.set_char('─');
                cell.set_style(Style::default().fg(theme.surface));
            }
        }
    }

    /// Renders stereo balance indicator and headroom stats at the bottom of the meter.
    fn render_stats(
        &self,
        buf: &mut ratatui::buffer::Buffer,
        start_x: u16,
        y: u16,
        width: usize,
        audio: &AudioFrame,
        theme: &Theme,
    ) {
        let total_energy = audio.rms_left + audio.rms_right;
        let balance = if total_energy > 0.005 {
            ((audio.rms_right - audio.rms_left) / total_energy).clamp(-1.0, 1.0)
        } else {
            0.0
        };

        // 9-character balance meter: [ ◄──●──► ]
        // Index 0..8 with center at 4
        let dot_pos = ((balance + 1.0) * 4.0).round().clamp(0.0, 8.0) as usize;
        let mut balance_chars = ['◄', '─', '─', '─', '┼', '─', '─', '─', '►'];
        balance_chars[dot_pos] = '●';
        let balance_str: String = balance_chars.into_iter().collect();

        let max_peak = audio.peak_left.max(audio.peak_right);
        let headroom_db = -Self::to_db(max_peak);
        let stats_line = format!("Bal: [ {} ]  Headroom: {:>4.1} dB", balance_str, headroom_db.max(0.0));

        let available = width.saturating_sub(2);
        for (i, ch) in stats_line.chars().take(available).enumerate() {
            let cell_x = start_x + 2 + i as u16;
            if let Some(cell) = buf.cell_mut((cell_x, y)) {
                cell.set_char(ch);
                cell.set_style(Style::default().fg(theme.text_dim));
            }
        }
    }
}
