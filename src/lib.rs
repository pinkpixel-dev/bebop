pub mod app;
pub mod audio;
pub mod config;
pub mod event;
pub mod library;
pub mod lyrics;
pub mod player;
pub mod terminal;
pub mod theme;
pub mod ui;
pub mod visualizers;

pub use app::App;
pub use config::AppConfig;
pub use lyrics::{LyricLine, Lyrics, LyricsSource, LyricsState};
