use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use crate::library::{MetadataReader, Track};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaylistInfo {
    pub name: String,
    pub path: PathBuf,
    pub track_count: usize,
}

pub struct PlaylistManager;

impl PlaylistManager {
    /// Directory where user-saved playlists are stored (~/.config/bebop/playlists/)
    pub fn playlists_dir() -> PathBuf {
        if let Some(config_dir) = dirs::config_dir() {
            config_dir.join("bebop").join("playlists")
        } else {
            PathBuf::from(".bebop_playlists")
        }
    }

    /// List all saved playlists in the user config directory
    pub fn list_saved_playlists() -> Vec<PlaylistInfo> {
        let dir = Self::playlists_dir();
        if !dir.exists() {
            return Vec::new();
        }

        let mut playlists = Vec::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if let Some(ext) = p.extension().and_then(|s| s.to_str()).map(|s| s.to_lowercase()) {
                    if ext == "m3u" || ext == "m3u8" {
                        let name = p.file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("Untitled")
                            .to_string();

                        // Count tracks quickly by reading lines
                        let track_count = Self::count_m3u_tracks(&p);
                        playlists.push(PlaylistInfo {
                            name,
                            path: p,
                            track_count,
                        });
                    }
                }
            }
        }

        playlists.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        playlists
    }

    /// Quick count of audio entries in an M3U file
    fn count_m3u_tracks(path: &Path) -> usize {
        if let Ok(file) = File::open(path) {
            let reader = BufReader::new(file);
            reader.lines()
                .filter_map(|l| l.ok())
                .filter(|line| {
                    let trimmed = line.trim();
                    !trimmed.is_empty() && !trimmed.starts_with('#')
                })
                .count()
        } else {
            0
        }
    }

    /// Load tracks from an M3U or M3U8 file
    pub fn load_m3u<P: AsRef<Path>>(path: P) -> Result<Vec<Track>, anyhow::Error> {
        let path = path.as_ref();
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let base_dir = path.parent().unwrap_or_else(|| Path::new("."));

        let mut tracks = Vec::new();

        for line_res in reader.lines() {
            let line = line_res?;
            let trimmed = line.trim();

            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Resolve track path (either absolute or relative to playlist file)
            let track_path = if Path::new(trimmed).is_absolute() {
                PathBuf::from(trimmed)
            } else {
                base_dir.join(trimmed)
            };

            if track_path.exists() && track_path.is_file() {
                let (track, _) = MetadataReader::read_track(&track_path);
                tracks.push(track);
            }
        }

        Ok(tracks)
    }

    /// Save tracks to an M3U file with extended metadata tags
    pub fn save_m3u<P: AsRef<Path>>(path: P, tracks: &[Track]) -> Result<(), anyhow::Error> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = File::create(path)?;
        writeln!(file, "#EXTM3U")?;

        for track in tracks {
            let secs = track.duration.as_secs();
            writeln!(file, "#EXTINF:{},{} - {}", secs, track.artist, track.title)?;
            writeln!(file, "{}", track.path.display())?;
        }

        Ok(())
    }

    /// Save active tracks as a named playlist in ~/.config/bebop/playlists/<name>.m3u
    pub fn save_named_playlist(name: &str, tracks: &[Track]) -> Result<PathBuf, anyhow::Error> {
        let dir = Self::playlists_dir();
        fs::create_dir_all(&dir)?;

        let sanitized: String = name
            .chars()
            .map(|c| if c.is_alphanumeric() || c == ' ' || c == '_' || c == '-' { c } else { '_' })
            .collect();

        let filename = format!("{}.m3u", sanitized.trim());
        let target_path = dir.join(filename);
        Self::save_m3u(&target_path, tracks)?;

        Ok(target_path)
    }

    /// Delete a saved playlist
    pub fn delete_playlist<P: AsRef<Path>>(path: P) -> Result<(), anyhow::Error> {
        fs::remove_file(path)?;
        Ok(())
    }
}
