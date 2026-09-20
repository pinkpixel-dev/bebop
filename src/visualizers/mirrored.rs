use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::Frame;

use crate::audio::frame::AudioFrame;
use crate::theme::Theme;
use crate::visualizers::Visualizer;

const BLOCKS: [char; 9] = [' ', ' ', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Mirrored spectrum bars visualizer.
/// Anchors low/bass frequencies at the center and mirrors outward to treble on the left and right wings.
pub struct MirroredBarsVisualizer {
    peaks: Vec<f32>,
}

impl Default for MirroredBarsVisualizer {
    fn default() -> Self {
        Self { peaks: Vec::new() }
    }
}

impl Visualizer for MirroredBarsVisualizer {
    fn name(&self) -> &'static str {
        "Mirrored Bars"
    }

    fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        audio: &AudioFrame,
        theme: &Theme,
    ) {
        if area.width < 2 || area.height < 1 {
            return;
        }

        let width = area.width as usize;
        let height = area.height as usize;
        let buf = frame.buffer_mut();

        if self.peaks.len() != width {
            self.peaks.resize(width, 0.0);
        }

        let spec_len = audio.spectrum.len();
        if spec_len == 0 {
            return;
        }

        let half_w = width / 2;
        let is_odd = width % 2 != 0;

        // Render each column x across the full width
        for x in 0..width {
            // Determine distance from center (0.0 = center/bass, 1.0 = outer edge/treble)
            let dist_from_center = if is_odd {
                let center_col = half_w;
                if x == center_col {
                    0.0
                } else if x < center_col {
                    let idx = center_col - x;
                    (idx as f32 / half_w.max(1) as f32).min(1.0)
                } else {
                    let idx = x - center_col;
                    (idx as f32 / half_w.max(1) as f32).min(1.0)
                }
            } else {
                if x < half_w {
                    let idx = half_w - 1 - x;
                    (idx as f32 / half_w.max(1) as f32).min(1.0)
                } else {
                    let idx = x - half_w;
                    (idx as f32 / half_w.max(1) as f32).min(1.0)
                }
            };

            // Map distance from center to frequency bin (bass at center, treble at edges)
            let spec_pos = dist_from_center * (spec_len - 1) as f32;
            let idx0 = spec_pos.floor() as usize;
            let idx1 = (idx0 + 1).min(spec_len - 1);
            let frac = spec_pos - idx0 as f32;
            let val = (audio.spectrum[idx0] * (1.0 - frac) + audio.spectrum[idx1] * frac).clamp(0.0, 1.0);

            // Update peak cap with gravity decay
            if val >= self.peaks[x] {
                self.peaks[x] = val;
            } else {
                self.peaks[x] = (self.peaks[x] - 0.025).max(0.0);
            }

            let bar_eighths = (val * height as f32 * 8.0).round() as usize;
            let peak_row = if self.peaks[x] > 0.05 {
                let r = (self.peaks[x] * height as f32).round() as usize;
                Some(r.min(height))
            } else {
                None
            };

            for row_from_bottom in 1..=height {
                let screen_y = area.bottom().saturating_sub(row_from_bottom as u16);
                let screen_x = area.left() + x as u16;

                let row_eighths_start = (row_from_bottom - 1) * 8;
                let cell = match buf.cell_mut((screen_x, screen_y)) {
                    Some(c) => c,
                    None => continue,
                };

                // Peak cap rendering
                if let Some(pk) = peak_row {
                    if pk == row_from_bottom && bar_eighths < row_eighths_start {
                        cell.set_char('▔');
                        cell.set_style(Style::default().fg(theme.visualizer_peak).add_modifier(Modifier::BOLD));
                        continue;
                    }
                }

                // Theme color based on frequency distance from center (bass = 0.0 at center, treble = 1.0 at wings)
                let color = theme.get_bar_color(dist_from_center, row_from_bottom as f32 / height as f32);

                if bar_eighths >= row_from_bottom * 8 {
                    cell.set_char(BLOCKS[8]);
                    cell.set_style(Style::default().fg(color));
                } else if bar_eighths > row_eighths_start {
                    let sub = bar_eighths - row_eighths_start;
                    cell.set_char(BLOCKS[sub.min(8)]);
                    cell.set_style(Style::default().fg(color));
                } else {
                    cell.set_char(' ');
                    cell.set_style(Style::default());
                }
            }
        }
    }
}
