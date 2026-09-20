use std::time::Duration;
use crate::audio::PlaybackState;
use crate::library::Track;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepeatMode {
    Off,
    All,
    One,
}

impl RepeatMode {
    pub fn cycle(&self) -> Self {
        match self {
            Self::Off => Self::All,
            Self::All => Self::One,
            Self::One => Self::Off,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Off => "Repeat Off",
            Self::All => "Repeat All",
            Self::One => "Repeat One",
        }
    }
}

#[derive(Clone, Debug)]
pub struct PlayerState {
    pub current_track: Option<Track>,
    pub playback_state: PlaybackState,
    pub position: Duration,
    pub duration: Duration,
    pub volume: f32,
    pub muted: bool,
    pub repeat: RepeatMode,
    pub shuffle: bool,
    pub device_name: Option<String>,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            current_track: None,
            playback_state: PlaybackState::Stopped,
            position: Duration::ZERO,
            duration: Duration::ZERO,
            volume: 0.8,
            muted: false,
            repeat: RepeatMode::All,
            shuffle: false,
            device_name: None,
        }
    }
}
