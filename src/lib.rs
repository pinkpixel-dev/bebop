pub mod app;
pub mod audio;
pub mod config;
pub mod event;
pub mod library;
pub mod lyrics;
pub mod mpris;
pub mod notifications;
pub mod player;
pub mod terminal;
pub mod theme;
pub mod ui;
pub mod visualizers;

pub use app::App;
pub use config::AppConfig;
pub use lyrics::{LyricLine, Lyrics, LyricsSource, LyricsState};
pub use mpris::{MprisAction, MprisService, MprisState};
pub use notifications::NotificationManager;
