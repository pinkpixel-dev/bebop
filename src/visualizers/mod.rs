pub mod bars;
pub mod waveform;

use ratatui::layout::Rect;
use ratatui::Frame;
use crate::audio::frame::AudioFrame;
use crate::theme::Theme;

pub trait Visualizer: Send + Sync {
    fn name(&self) -> &'static str;

    fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        audio: &AudioFrame,
        theme: &Theme,
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisualizerKind {
    Bars,
    Waveform,
}

impl VisualizerKind {
    pub fn next(&self) -> Self {
        match self {
            Self::Bars => Self::Waveform,
            Self::Waveform => Self::Bars,
        }
    }

    #[allow(dead_code)]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Bars => "Spectrum Bars",
            Self::Waveform => "Waveform",
        }
    }
}
