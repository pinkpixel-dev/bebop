use std::collections::VecDeque;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::Frame;

use crate::audio::frame::AudioFrame;
use crate::theme::palette::interpolate_color;
use crate::theme::Theme;
use crate::visualizers::Visualizer;

const DENSITY_CHARS: [char; 6] = [' ', '·', '░', '▒', '▓', '█'];

/// Energy below this reads as background. The dB-scaled spectrum keeps ordinary
/// room tone and reverb tails well off zero, so the display needs its own floor
/// to stay dark where nothing is happening.
const DISPLAY_FLOOR: f32 = 0.25;
/// Contrast curve applied above the floor. Higher values darken the midrange
/// and leave the bright ridges to real peaks.
const DISPLAY_GAMMA: f32 = 1.4;
/// Intensity at which a cell reaches its full theme hue. Below this it is mixed
/// back toward the background, which is what carries quiet detail.
const FULL_TINT_AT: f32 = 0.55;
/// Intensity at which a cell starts blowing out toward the peak color.
const HOT_FROM: f32 = 0.78;
/// Strongest push toward the peak color at full intensity.
const HOT_MIX: f32 = 0.60;

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

                // Lift the display floor and add contrast, so quiet cells fall
                // back to the background instead of painting a solid sheet.
                let intensity = ((energy - DISPLAY_FLOOR) / (1.0 - DISPLAY_FLOOR))
                    .clamp(0.0, 1.0)
                    .powf(DISPLAY_GAMMA);

                if intensity < 0.05 {
                    cell.set_char(' ');
                    cell.set_style(Style::default());
                    continue;
                }

                let char_idx = if intensity < 0.16 {
                    1 // '·'
                } else if intensity < 0.32 {
                    2 // '░'
                } else if intensity < 0.52 {
                    3 // '▒'
                } else if intensity < 0.75 {
                    4 // '▓'
                } else {
                    5 // '█'
                };

                // Hue still comes from frequency, but intensity decides how much
                // of it survives: background at the bottom of the range, full
                // theme color in the middle, peak color at the top.
                let base = theme.get_bar_color(freq_t, intensity);
                let tint = (intensity / FULL_TINT_AT).min(1.0);
                let mut color = interpolate_color(theme.bg, base, tint);

                if intensity > HOT_FROM {
                    let heat = ((intensity - HOT_FROM) / (1.0 - HOT_FROM)) * HOT_MIX;
                    color = interpolate_color(color, theme.visualizer_peak, heat);
                }

                let mut style = Style::default().fg(color);
                if intensity >= 0.90 {
                    style = style.add_modifier(Modifier::BOLD);
                }

                cell.set_char(DENSITY_CHARS[char_idx]);
                cell.set_style(style);
            }
        }
    }
}
