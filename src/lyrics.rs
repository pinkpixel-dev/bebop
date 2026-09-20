use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use lofty::file::TaggedFileExt;
use lofty::probe::Probe;
use lofty::tag::ItemKey;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LyricLine {
    pub timestamp: Duration,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LyricsSource {
    None,
    CompanionFile(PathBuf),
    EmbeddedTag,
}

impl Default for LyricsSource {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lyrics {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub offset_ms: i64,
    pub lines: Vec<LyricLine>,
    pub source: LyricsSource,
}

impl Lyrics {
    /// Parse LRC format string into a structured `Lyrics` instance.
    pub fn parse(content: &str) -> Self {
        let mut title = None;
        let mut artist = None;
        let mut album = None;
        let mut offset_ms: i64 = 0;
        let mut raw_lines = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Check metadata tags e.g. [ti:Song Title], [ar:Artist], [al:Album], [offset:+/-500]
            if trimmed.starts_with("[ti:") && trimmed.ends_with(']') {
                title = Some(trimmed[4..trimmed.len() - 1].trim().to_string());
                continue;
            }
            if trimmed.starts_with("[ar:") && trimmed.ends_with(']') {
                artist = Some(trimmed[4..trimmed.len() - 1].trim().to_string());
                continue;
            }
            if trimmed.starts_with("[al:") && trimmed.ends_with(']') {
                album = Some(trimmed[4..trimmed.len() - 1].trim().to_string());
                continue;
            }
            if trimmed.starts_with("[offset:") && trimmed.ends_with(']') {
                if let Ok(val) = trimmed[8..trimmed.len() - 1].trim().parse::<i64>() {
                    offset_ms = val;
                }
                continue;
            }

            // Extract all timestamps from line (e.g. "[00:12.00][00:25.00] Chorus text")
            let mut remaining = trimmed;
            let mut line_timestamps = Vec::new();

            while let Some(open_idx) = remaining.find('[') {
                if open_idx > 0 && line_timestamps.is_empty() {
                    // Bracket is not at the start
                    break;
                }
                if let Some(close_idx) = remaining[open_idx..].find(']') {
                    let full_close = open_idx + close_idx;
                    let tag_content = &remaining[open_idx + 1..full_close];
                    if let Some(ts) = parse_timestamp(tag_content) {
                        line_timestamps.push(ts);
                        remaining = remaining[full_close + 1..].trim_start();
                    } else {
                        // Not a valid timestamp tag, move on
                        break;
                    }
                } else {
                    break;
                }
            }

            let text = remaining.trim().to_string();
            for ts in line_timestamps {
                raw_lines.push(LyricLine {
                    timestamp: ts,
                    text: text.clone(),
                });
            }
        }

        // Apply offset if present
        if offset_ms != 0 {
            for line in &mut raw_lines {
                let current_ms = line.timestamp.as_millis() as i64;
                let adjusted = (current_ms + offset_ms).max(0);
                line.timestamp = Duration::from_millis(adjusted as u64);
            }
        }

        // Sort lines chronologically
        raw_lines.sort_by_key(|l| l.timestamp);

        Self {
            title,
            artist,
            album,
            offset_ms,
            lines: raw_lines,
            source: LyricsSource::None,
        }
    }

    /// Find the index of the currently active lyric line for the given playback position.
    /// Returns `None` if playback is before the first timestamp or if there are no lines.
    pub fn find_active_index(&self, position: Duration) -> Option<usize> {
        if self.lines.is_empty() || position < self.lines[0].timestamp {
            return None;
        }

        let mut active = 0;
        for (i, line) in self.lines.iter().enumerate() {
            if line.timestamp <= position {
                active = i;
            } else {
                break;
            }
        }
        Some(active)
    }

    /// Discover and load synchronized lyrics for a target audio track.
    /// Checks companion `<track_stem>.lrc` file first, then falls back to embedded lyrics.
    pub fn load_for_track<P: AsRef<Path>>(track_path: P) -> Option<Self> {
        let p = track_path.as_ref();

        // 1. Check for companion .lrc / .LRC file in same folder
        if let (Some(parent), Some(stem)) = (p.parent(), p.file_stem().and_then(|s| s.to_str())) {
            let lrc_candidates = [
                parent.join(format!("{}.lrc", stem)),
                parent.join(format!("{}.LRC", stem)),
            ];

            for candidate in &lrc_candidates {
                if candidate.exists() {
                    if let Ok(content) = fs::read_to_string(candidate) {
                        let mut lyrics = Self::parse(&content);
                        if !lyrics.lines.is_empty() {
                            lyrics.source = LyricsSource::CompanionFile(candidate.clone());
                            return Some(lyrics);
                        }
                    }
                }
            }
        }

        // 2. Check for embedded lyrics tag in the audio file using Lofty
        if let Ok(tagged_file) = Probe::open(p).and_then(|pr| pr.read()) {
            if let Some(tag) = tagged_file.primary_tag().or_else(|| tagged_file.first_tag()) {
                if let Some(text) = tag.get_string(&ItemKey::Lyrics) {
                    let mut lyrics = Self::parse(text);
                    if !lyrics.lines.is_empty() {
                        lyrics.source = LyricsSource::EmbeddedTag;
                        return Some(lyrics);
                    }
                }
            }
        }

        None
    }
}

/// Parse a timestamp tag string like "01:23.45", "01:23.456", or "01:23" into a `Duration`.
fn parse_timestamp(tag: &str) -> Option<Duration> {
    let parts: Vec<&str> = tag.split(':').collect();
    if parts.len() != 2 {
        return None;
    }

    let mins: u64 = parts[0].trim().parse().ok()?;
    let sec_str = parts[1].trim();

    if let Some(dot_idx) = sec_str.find('.') {
        let secs: u64 = sec_str[..dot_idx].parse().ok()?;
        let frac_str = &sec_str[dot_idx + 1..];

        let millis: u64 = match frac_str.len() {
            0 => 0,
            1 => frac_str.parse::<u64>().ok()? * 100,
            2 => frac_str.parse::<u64>().ok()? * 10,
            3 => frac_str.parse::<u64>().ok()?,
            _ => frac_str[..3].parse::<u64>().ok()?,
        };

        Some(Duration::from_millis(mins * 60 * 1000 + secs * 1000 + millis))
    } else {
        let secs: u64 = sec_str.parse().ok()?;
        Some(Duration::from_secs(mins * 60 + secs))
    }
}

#[derive(Clone, Debug, Default)]
pub struct LyricsState {
    pub lyrics: Option<Lyrics>,
    pub selected_line_idx: usize,
    pub auto_scroll: bool,
}

impl LyricsState {
    pub fn new() -> Self {
        Self {
            lyrics: None,
            selected_line_idx: 0,
            auto_scroll: true,
        }
    }

    pub fn set_lyrics(&mut self, lyrics: Option<Lyrics>) {
        self.lyrics = lyrics;
        self.selected_line_idx = 0;
        self.auto_scroll = true;
    }

    pub fn move_up(&mut self) {
        if self.selected_line_idx > 0 {
            self.selected_line_idx -= 1;
            self.auto_scroll = false;
        }
    }

    pub fn move_down(&mut self, total_lines: usize) {
        if total_lines > 0 && self.selected_line_idx + 1 < total_lines {
            self.selected_line_idx += 1;
            self.auto_scroll = false;
        }
    }

    pub fn sync_active(&mut self, active_idx: Option<usize>) {
        if self.auto_scroll {
            if let Some(idx) = active_idx {
                self.selected_line_idx = idx;
            }
        }
    }

    pub fn resume_auto_scroll(&mut self, active_idx: Option<usize>) {
        self.auto_scroll = true;
        if let Some(idx) = active_idx {
            self.selected_line_idx = idx;
        }
    }

    pub fn selected_timestamp(&self) -> Option<Duration> {
        self.lyrics.as_ref().and_then(|l| l.lines.get(self.selected_line_idx)).map(|line| line.timestamp)
    }
}
