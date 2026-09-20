use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::Frame;

use crate::audio::frame::AudioFrame;
use crate::theme::Theme;
use crate::visualizers::Visualizer;

pub struct WaveformVisualizer;

impl Default for WaveformVisualizer {
    fn default() -> Self {
        Self
    }
}

impl Visualizer for WaveformVisualizer {
    fn name(&self) -> &'static str {
        "Waveform"
    }

    fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        audio: &AudioFrame,
        theme: &Theme,
    ) {
        if area.width < 2 || area.height < 2 {
            return;
        }

        let width = area.width as usize;
        let height = area.height as usize;
        let mid_y = area.top() + (area.height / 2);
        let buf = frame.buffer_mut();
        let samples = &audio.waveform;

        if samples.is_empty() {
            return;
        }

        for x in 0..width {
            let sample_pos = (x as f32 / width as f32) * (samples.len() - 1) as f32;
            let idx = sample_pos.round() as usize;
            let val = samples.get(idx).copied().unwrap_or(0.0).clamp(-1.0, 1.0);

            let half_h = (height as f32 * 0.45).max(1.0);
            let y_offset = (val * half_h).round() as i32;
            let target_y = (mid_y as i32 - y_offset).clamp(area.top() as i32, area.bottom().saturating_sub(1) as i32) as u16;
            let screen_x = area.left() + x as u16;

            // Clear column
            for y in area.top()..area.bottom() {
                if let Some(cell) = buf.cell_mut((screen_x, y)) {
                    cell.set_char(' ');
                    cell.set_style(Style::default());
                }
            }

            // Draw center baseline tick if idle
            if let Some(cell) = buf.cell_mut((screen_x, mid_y)) {
                cell.set_char('─');
                cell.set_style(Style::default().fg(theme.text_dim));
            }

            // Draw waveform point
            if let Some(cell) = buf.cell_mut((screen_x, target_y)) {
                let ch = if target_y == mid_y {
                    '━'
                } else if target_y < mid_y {
                    '▀'
                } else {
                    '▄'
                };
                let color = theme.get_bar_color(x as f32 / width as f32, 0.5);
                cell.set_char(ch);
                cell.set_style(Style::default().fg(color));
            }
        }
    }
}
