use std::time::{SystemTime, UNIX_EPOCH};

use crate::library::Track;

fn next_random_u64(state: &mut u64) -> u64 {
    if *state == 0 {
        *state = 0x9e3779b97f4a7c15;
    }
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

fn get_seed() -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    (now.as_secs() ^ (now.subsec_nanos() as u64) << 32) ^ 0xa0761d6478bd642f
}

#[derive(Clone, Debug, Default)]
pub struct Queue {
    pub tracks: Vec<Track>,
    pub current_index: Option<usize>,
    pub shuffle: bool,
    pub shuffle_order: Vec<usize>,
    pub shuffle_cursor: usize,
    pub history: Vec<usize>,
}

impl Queue {
    pub fn new() -> Self {
        Self {
            tracks: Vec::new(),
            current_index: None,
            shuffle: false,
            shuffle_order: Vec::new(),
            shuffle_cursor: 0,
            history: Vec::new(),
        }
    }

    /// Generate an unbiased Fisher-Yates permutation deck of length `len`.
    /// If `pin_first` is provided and in bounds, that index is pinned to position 0
    /// while the remaining `len - 1` elements are shuffled.
    pub fn generate_shuffle_deck(len: usize, pin_first: Option<usize>) -> Vec<usize> {
        if len == 0 {
            return Vec::new();
        }
        if len == 1 {
            return vec![0];
        }

        let mut order: Vec<usize> = (0..len).collect();
        let mut seed = get_seed();

        if let Some(pin) = pin_first {
            if pin < len {
                order.swap(0, pin);
                for i in (2..len).rev() {
                    let j = 1 + (next_random_u64(&mut seed) as usize % i);
                    order.swap(i, j);
                }
                return order;
            }
        }

        for i in (1..len).rev() {
            let j = next_random_u64(&mut seed) as usize % (i + 1);
            order.swap(i, j);
        }
        order
    }

    /// Toggle or set shuffle mode. When enabled, builds a Fisher-Yates permutation deck
    /// placing the currently playing track at position 0 so playback continues uninterrupted.
    pub fn set_shuffle(&mut self, enabled: bool) {
        if self.shuffle == enabled {
            return;
        }
        self.shuffle = enabled;
        let count = self.tracks.len();
        if enabled {
            if count > 0 {
                self.shuffle_order = Self::generate_shuffle_deck(count, self.current_index);
                self.shuffle_cursor = 0;
                if self.current_index.is_none() {
                    self.current_index = self.shuffle_order.first().copied();
                }
            } else {
                self.shuffle_order.clear();
                self.shuffle_cursor = 0;
            }
        } else if let Some(cur) = self.current_index {
            self.shuffle_cursor = cur;
        }
    }

    pub fn set_tracks(&mut self, tracks: Vec<Track>, start_index: usize) {
        let count = tracks.len();
        self.tracks = tracks;
        self.history.clear();
        if count > 0 {
            let start = start_index.min(count - 1);
            self.current_index = Some(start);
            if self.shuffle {
                self.shuffle_order = Self::generate_shuffle_deck(count, Some(start));
                self.shuffle_cursor = 0;
            } else {
                self.shuffle_order = (0..count).collect();
                self.shuffle_cursor = start;
            }
        } else {
            self.current_index = None;
            self.shuffle_order.clear();
            self.shuffle_cursor = 0;
        }
    }

    pub fn add_track(&mut self, track: Track) {
        self.tracks.push(track);
        let new_idx = self.tracks.len() - 1;
        if self.current_index.is_none() {
            self.current_index = Some(0);
            self.shuffle_order = vec![0];
            self.shuffle_cursor = 0;
        } else if self.shuffle {
            if self.shuffle_cursor + 1 < self.shuffle_order.len() {
                let mut seed = get_seed();
                let remaining = self.shuffle_order.len() - (self.shuffle_cursor + 1);
                let insert_pos = (self.shuffle_cursor + 1) + (next_random_u64(&mut seed) as usize % (remaining + 1));
                self.shuffle_order.insert(insert_pos, new_idx);
            } else {
                self.shuffle_order.push(new_idx);
            }
        } else {
            self.shuffle_order.push(new_idx);
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

        if self.shuffle {
            if self.shuffle_cursor + 1 < self.shuffle_order.len() {
                Some(self.shuffle_order[self.shuffle_cursor + 1])
            } else if repeat_all {
                self.shuffle_order.first().copied()
            } else {
                None
            }
        } else {
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
    }

    pub fn prev_index(&self) -> Option<usize> {
        let count = self.tracks.len();
        if count == 0 {
            return None;
        }

        if self.shuffle {
            if self.shuffle_cursor > 0 {
                Some(self.shuffle_order[self.shuffle_cursor - 1])
            } else {
                self.shuffle_order.first().copied()
            }
        } else {
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
    }

    pub fn advance_next(&mut self, repeat_all: bool) -> Option<&Track> {
        let count = self.tracks.len();
        if count == 0 {
            return None;
        }

        if self.shuffle {
            if self.shuffle_order.is_empty() {
                self.shuffle_order = Self::generate_shuffle_deck(count, self.current_index);
                self.shuffle_cursor = 0;
            }

            if self.shuffle_cursor + 1 < self.shuffle_order.len() {
                if let Some(cur) = self.current_index {
                    self.history.push(cur);
                }
                self.shuffle_cursor += 1;
                let next_idx = self.shuffle_order[self.shuffle_cursor];
                self.current_index = Some(next_idx);
                self.current_track()
            } else if repeat_all {
                if let Some(cur) = self.current_index {
                    self.history.push(cur);
                }
                let last_played = self.current_index;
                let mut new_deck = Self::generate_shuffle_deck(count, None);
                if count > 1 && new_deck.first().copied() == last_played {
                    new_deck.swap(0, 1);
                }
                self.shuffle_order = new_deck;
                self.shuffle_cursor = 0;
                let next_idx = self.shuffle_order[0];
                self.current_index = Some(next_idx);
                self.current_track()
            } else {
                None
            }
        } else if let Some(next) = self.next_index(repeat_all) {
            if let Some(cur) = self.current_index {
                self.history.push(cur);
            }
            self.current_index = Some(next);
            self.current_track()
        } else {
            None
        }
    }

    pub fn advance_prev(&mut self) -> Option<&Track> {
        let count = self.tracks.len();
        if count == 0 {
            return None;
        }

        if self.shuffle {
            if self.shuffle_cursor > 0 {
                self.shuffle_cursor -= 1;
                let prev_idx = self.shuffle_order[self.shuffle_cursor];
                self.current_index = Some(prev_idx);
                self.current_track()
            } else {
                self.current_track()
            }
        } else if let Some(prev) = self.prev_index() {
            self.current_index = Some(prev);
            self.current_track()
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn jump_to(&mut self, index: usize) -> Option<&Track> {
        if index < self.tracks.len() {
            if let Some(cur) = self.current_index {
                self.history.push(cur);
            }
            self.current_index = Some(index);
            if self.shuffle {
                if let Some(pos) = self.shuffle_order.iter().position(|&x| x == index) {
                    if pos >= self.shuffle_cursor {
                        self.shuffle_order.swap(self.shuffle_cursor, pos);
                    } else {
                        self.shuffle_cursor = pos;
                    }
                }
            }
            self.current_track()
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn remove_at(&mut self, index: usize) {
        if index >= self.tracks.len() {
            return;
        }
        self.tracks.remove(index);

        if let Some(pos) = self.shuffle_order.iter().position(|&x| x == index) {
            self.shuffle_order.remove(pos);
            if pos < self.shuffle_cursor && self.shuffle_cursor > 0 {
                self.shuffle_cursor -= 1;
            }
        }
        for idx in &mut self.shuffle_order {
            if *idx > index {
                *idx -= 1;
            }
        }

        self.history.retain(|&x| x != index);
        for idx in &mut self.history {
            if *idx > index {
                *idx -= 1;
            }
        }

        if self.tracks.is_empty() {
            self.current_index = None;
            self.shuffle_cursor = 0;
        } else if self.shuffle {
            if self.shuffle_cursor >= self.shuffle_order.len() {
                self.shuffle_cursor = self.shuffle_order.len().saturating_sub(1);
            }
            self.current_index = self.shuffle_order.get(self.shuffle_cursor).copied();
        } else if let Some(cur) = self.current_index {
            if cur == index {
                self.current_index = Some(cur.min(self.tracks.len() - 1));
            } else if cur > index {
                self.current_index = Some(cur - 1);
            }
        }
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.tracks.clear();
        self.current_index = None;
        self.shuffle_order.clear();
        self.shuffle_cursor = 0;
        self.history.clear();
    }
}
