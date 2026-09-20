use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::Frame;

use crate::audio::frame::AudioFrame;
use crate::theme::Theme;
use crate::visualizers::Visualizer;

const BLOCKS: [char; 9] = [' ', ' ', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

pub struct BarsVisualizer {
    peaks: Vec<f32>,
}

impl Default for BarsVisualizer {
    fn default() -> Self {
        Self { peaks: Vec::new() }
    }
}

impl Visualizer for BarsVisualizer {
    fn name(&self) -> &'static str {
        "Spectrum Bars"
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

        // Draw each vertical column
        for x in 0..width {
            // Map column x to spectrum bin index with linear interpolation
            let spec_pos = (x as f32 / width as f32) * (spec_len - 1) as f32;
            let idx0 = spec_pos.floor() as usize;
            let idx1 = (idx0 + 1).min(spec_len - 1);
            let frac = spec_pos - idx0 as f32;
            let val = (audio.spectrum[idx0] * (1.0 - frac) + audio.spectrum[idx1] * frac).clamp(0.0, 1.0);

            // Update peak cap
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
                let cell = buf.cell_mut((screen_x, screen_y));
                if cell.is_none() {
                    continue;
                }
                let cell = cell.unwrap();

                // Peak cap rendering
                if let Some(pk) = peak_row {
                    if pk == row_from_bottom && bar_eighths < row_eighths_start {
                        cell.set_char('▔');
                        cell.set_style(Style::default().fg(theme.visualizer_peak).add_modifier(Modifier::BOLD));
                        continue;
                    }
                }

                let color = theme.get_bar_color(x as f32 / width as f32, row_from_bottom as f32 / height as f32);

                if bar_eighths >= row_from_bottom * 8 {
                    // Full block
                    cell.set_char(BLOCKS[8]);
                    cell.set_style(Style::default().fg(color));
                } else if bar_eighths > row_eighths_start {
                    // Partial block
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
