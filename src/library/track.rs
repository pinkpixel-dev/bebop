use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Track {
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub track_number: Option<u32>,
    pub duration: Duration,
    pub format: String,
    pub sample_rate: u32,
    pub bit_depth: Option<u8>,
    pub channels: usize,
    pub has_artwork: bool,
}

impl Track {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let p = path.as_ref().to_path_buf();
        let fallback_title = p.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown Track")
            .to_string();

        let ext = p.extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_uppercase())
            .unwrap_or_else(|| "AUDIO".to_string());

        Self {
            path: p,
            title: fallback_title,
            artist: "Unknown Artist".to_string(),
            album: "Unknown Album".to_string(),
            track_number: None,
            duration: Duration::ZERO,
            format: ext,
            sample_rate: 44100,
            bit_depth: Some(16),
            channels: 2,
            has_artwork: false,
        }
    }

    /// Human readable track duration format (MM:SS or HH:MM:SS)
    #[allow(dead_code)]
    pub fn formatted_duration(&self) -> String {
        format_time(self.duration)
    }

    /// Formatted audio tech details (e.g. "FLAC · 44.1 kHz · 24-bit")
    pub fn formatted_tech_details(&self) -> String {
        let khz = (self.sample_rate as f64 / 1000.0 * 10.0).round() / 10.0;
        let bits = self.bit_depth.map(|b| format!(" · {}-bit", b)).unwrap_or_default();
        format!("{} · {:.1} kHz{}", self.format, khz, bits)
    }
}

pub fn format_time(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, mins, secs)
    } else {
        format!("{:02}:{:02}", mins, secs)
    }
}
