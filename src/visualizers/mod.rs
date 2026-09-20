pub mod bars;
pub mod mirrored;
pub mod vu;
pub mod waterfall;
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
    Mirrored,
    Waveform,
    VuMeter,
    Waterfall,
}

impl VisualizerKind {
    pub fn next(&self) -> Self {
        match self {
            Self::Bars => Self::Mirrored,
            Self::Mirrored => Self::Waveform,
            Self::Waveform => Self::VuMeter,
            Self::VuMeter => Self::Waterfall,
            Self::Waterfall => Self::Bars,
        }
    }

    #[allow(dead_code)]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Bars => "Spectrum Bars",
            Self::Mirrored => "Mirrored Bars",
            Self::Waveform => "Waveform",
            Self::VuMeter => "Stereo VU Meter",
            Self::Waterfall => "Waterfall Spectrogram",
        }
    }
}
