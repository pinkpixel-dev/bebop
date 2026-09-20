use std::thread;
use notify_rust::{Notification, Timeout};
use crate::library::Track;

pub struct NotificationManager;

impl NotificationManager {
    /// Format the notification summary (title) and body text for a track.
    pub fn format_payload(track: &Track) -> (String, String) {
        let summary = track.title.clone();
        let duration_str = track.formatted_duration();
        let body = if !track.album.is_empty() && track.album != "Unknown Album" {
            format!("{} • {}\n{}", track.artist, track.album, duration_str)
        } else {
            format!("{}\n{}", track.artist, duration_str)
        };
        (summary, body)
    }

    /// Dispatch a desktop notification in a detached background thread.
    /// This guarantees zero latency impact on UI rendering and audio playback.
    /// Silently ignores errors if D-Bus or notification server is unavailable.
    pub fn send_track_notification(track: &Track) {
        let (summary, body) = Self::format_payload(track);

        thread::spawn(move || {
            let _ = Notification::new()
                .appname("Auri")
                .summary(&summary)
                .body(&body)
                .icon("audio-x-generic")
                .timeout(Timeout::Milliseconds(4000))
                .show();
        });
    }
}
