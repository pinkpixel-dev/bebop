use std::path::Path;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;
use crate::library::track::Track;

pub struct MetadataReader;

impl MetadataReader {
    pub fn read_track<P: AsRef<Path>>(path: P) -> (Track, Option<Vec<u8>>) {
        let p = path.as_ref();
        let mut track = Track::new(p);
        let mut artwork_data: Option<Vec<u8>> = None;

        if let Ok(tagged_file) = Probe::open(p).and_then(|pr| pr.read()) {
            let props = tagged_file.properties();
            track.duration = props.duration();
            if let Some(rate) = props.sample_rate() {
                track.sample_rate = rate;
            }
            track.bit_depth = props.bit_depth();
            track.channels = props.channels().unwrap_or(2) as usize;

            if let Some(tag) = tagged_file.primary_tag().or_else(|| tagged_file.first_tag()) {
                if let Some(title) = tag.title() {
                    let t = title.trim();
                    if !t.is_empty() {
                        track.title = t.to_string();
                    }
                }
                if let Some(artist) = tag.artist() {
                    let a = artist.trim();
                    if !a.is_empty() {
                        track.artist = a.to_string();
                    }
                }
                if let Some(album) = tag.album() {
                    let al = album.trim();
                    if !al.is_empty() {
                        track.album = al.to_string();
                    }
                }
                track.track_number = tag.track();

                // Look for embedded cover art
                for pic in tag.pictures() {
                    let data = pic.data();
                    if !data.is_empty() {
                        artwork_data = Some(data.to_vec());
                        track.has_artwork = true;
                        break;
                    }
                }
            }
        }

        // If no embedded artwork found, look in the track directory for cover.jpg / cover.png / folder.jpg
        if artwork_data.is_none() {
            if let Some(parent) = p.parent() {
                let candidates = ["cover.jpg", "cover.png", "folder.jpg", "front.jpg", "artwork.jpg", "artwork.png"];
                for c in &candidates {
                    let cover_path = parent.join(c);
                    if cover_path.exists() {
                        if let Ok(bytes) = std::fs::read(&cover_path) {
                            artwork_data = Some(bytes);
                            track.has_artwork = true;
                            break;
                        }
                    }
                }
            }
        }

        (track, artwork_data)
    }
}
