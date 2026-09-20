use crate::library::Track;

#[derive(Clone, Debug, Default)]
pub struct Queue {
    pub tracks: Vec<Track>,
    pub current_index: Option<usize>,
}

impl Queue {
    pub fn new() -> Self {
        Self {
            tracks: Vec::new(),
            current_index: None,
        }
    }

    pub fn set_tracks(&mut self, tracks: Vec<Track>, start_index: usize) {
        let count = tracks.len();
        self.tracks = tracks;
        self.current_index = if count > 0 {
            Some(start_index.min(count - 1))
        } else {
            None
        };
    }

    pub fn add_track(&mut self, track: Track) {
        self.tracks.push(track);
        if self.current_index.is_none() {
            self.current_index = Some(0);
        }
    }

    pub fn current_track(&self) -> Option<&Track> {
        self.current_index.and_then(|idx| self.tracks.get(idx))
    }

    pub fn next_index(&self, repeat_all: bool) -> Option<usize> {
        let count = self.tracks.len();
        if count == 0 {
            return None;
        }

        match self.current_index {
            Some(idx) => {
                if idx + 1 < count {
                    Some(idx + 1)
                } else if repeat_all {
                    Some(0)
                } else {
                    None
                }
            }
            None => Some(0),
        }
    }

    pub fn prev_index(&self) -> Option<usize> {
        let count = self.tracks.len();
        if count == 0 {
            return None;
        }

        match self.current_index {
            Some(idx) => {
                if idx > 0 {
                    Some(idx - 1)
                } else {
                    Some(0)
                }
            }
            None => Some(0),
        }
    }

    pub fn advance_next(&mut self, repeat_all: bool) -> Option<&Track> {
        if let Some(next) = self.next_index(repeat_all) {
            self.current_index = Some(next);
            self.current_track()
        } else {
            None
        }
    }

    pub fn advance_prev(&mut self) -> Option<&Track> {
        if let Some(prev) = self.prev_index() {
            self.current_index = Some(prev);
            self.current_track()
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn jump_to(&mut self, index: usize) -> Option<&Track> {
        if index < self.tracks.len() {
            self.current_index = Some(index);
            self.current_track()
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn remove_at(&mut self, index: usize) {
        if index < self.tracks.len() {
            self.tracks.remove(index);
            if let Some(cur) = self.current_index {
                if self.tracks.is_empty() {
                    self.current_index = None;
                } else if cur >= self.tracks.len() {
                    self.current_index = Some(self.tracks.len() - 1);
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.tracks.clear();
        self.current_index = None;
    }
}
