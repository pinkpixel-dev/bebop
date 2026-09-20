use std::collections::VecDeque;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::Frame;

use crate::audio::frame::AudioFrame;
use crate::theme::Theme;
use crate::visualizers::Visualizer;

const DENSITY_CHARS: [char; 6] = [' ', '·', '░', '▒', '▓', '█'];

/// Waterfall / Spectrogram visualizer.
/// Renders a rolling time-history of frequency spectrum frames moving downward vertically.
pub struct WaterfallVisualizer {
    history: VecDeque<Vec<f32>>,
    last_width: usize,
}

impl Default for WaterfallVisualizer {
    fn default() -> Self {
        Self {
            history: VecDeque::new(),
            last_width: 0,
        }
    }
}

impl Visualizer for WaterfallVisualizer {
    fn name(&self) -> &'static str {
        "Waterfall Spectrogram"
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

        // Reset history if terminal width changed
        if width != self.last_width {
            self.history.clear();
            self.last_width = width;
        }

        let spec_len = audio.spectrum.len();
        if spec_len == 0 {
            return;
        }

        // Interpolate current audio spectrum across the visualizer width
        let mut current_row = Vec::with_capacity(width);
        for x in 0..width {
            let spec_pos = (x as f32 / width as f32) * (spec_len - 1) as f32;
            let idx0 = spec_pos.floor() as usize;
            let idx1 = (idx0 + 1).min(spec_len - 1);
            let frac = spec_pos - idx0 as f32;
            let val = (audio.spectrum[idx0] * (1.0 - frac) + audio.spectrum[idx1] * frac).clamp(0.0, 1.0);
            current_row.push(val);
        }

        // Push new frequency row at top (flows downward)
        self.history.push_front(current_row);
        while self.history.len() > height {
            self.history.pop_back();
        }

        // Render each historical row from top (newest) to bottom (oldest)
        for (row_idx, row) in self.history.iter().enumerate() {
            let screen_y = area.top() + row_idx as u16;

            for x in 0..width {
                let screen_x = area.left() + x as u16;
                let cell = match buf.cell_mut((screen_x, screen_y)) {
                    Some(c) => c,
                    None => continue,
                };

                let energy = row[x];
                let freq_t = x as f32 / width as f32;

                if energy < 0.04 {
                    cell.set_char(' ');
                    cell.set_style(Style::default());
                } else if energy < 0.15 {
                    cell.set_char(DENSITY_CHARS[1]); // '·'
                    cell.set_style(Style::default().fg(theme.text_dim));
                } else {
                    let char_idx = if energy < 0.35 {
                        2 // '░'
                    } else if energy < 0.60 {
                        3 // '▒'
                    } else if energy < 0.85 {
                        4 // '▓'
                    } else {
                        5 // '█'
                    };

                    let color = theme.get_bar_color(freq_t, energy);
                    let mut style = Style::default().fg(color);
                    if energy >= 0.90 {
                        style = style.add_modifier(Modifier::BOLD);
                    }

                    cell.set_char(DENSITY_CHARS[char_idx]);
                    cell.set_style(style);
                }
            }
        }
    }
}
