use std::path::Path;
use crate::library::Track;

/// Computes a fuzzy match score between target text and query string.
/// Returns None if query is not a subsequence of text.
pub fn fuzzy_score(text: &str, query: &str) -> Option<i64> {
    if query.is_empty() {
        return Some(0);
    }

    let text_lower: Vec<char> = text.to_lowercase().chars().collect();
    let query_lower: Vec<char> = query.to_lowercase().chars().collect();

    if query_lower.len() > text_lower.len() {
        return None;
    }

    let mut score = 100i64;
    let mut text_idx = 0;
    let mut prev_matched_idx: Option<usize> = None;

    for &q_char in &query_lower {
        let mut matched = false;
        while text_idx < text_lower.len() {
            let t_char = text_lower[text_idx];
            if t_char == q_char {
                // Base match points
                score += 20;

                // Word boundary bonus (start of string or follows space, underscore, dash, slash, dot)
                if text_idx == 0 || [' ', '_', '-', '/', '.', '(', '['].contains(&text_lower[text_idx - 1]) {
                    score += 35;
                }

                // Consecutive character match bonus
                if let Some(prev) = prev_matched_idx {
                    if text_idx == prev + 1 {
                        score += 25;
                    } else {
                        // Distance penalty
                        score -= (text_idx - prev) as i64 * 2;
                    }
                } else if text_idx == 0 {
                    // Exact prefix match bonus
                    score += 50;
                }

                prev_matched_idx = Some(text_idx);
                text_idx += 1;
                matched = true;
                break;
            }
            text_idx += 1;
        }

        if !matched {
            return None;
        }
    }

    // Exact substring bonus
    if text.to_lowercase().contains(&query.to_lowercase()) {
        score += 80;
    }

    // Slight penalty for length difference to favor tighter matches
    score -= (text_lower.len().saturating_sub(query_lower.len())) as i64;

    Some(score)
}

/// Evaluates a track against query across title, artist, album, and filename.
pub fn score_track(track: &Track, query: &str) -> Option<i64> {
    if query.is_empty() {
        return Some(0);
    }

    let filename = Path::new(&track.path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    let title_score = fuzzy_score(&track.title, query).map(|s| s * 3);
    let artist_score = fuzzy_score(&track.artist, query).map(|s| s * 2);
    let album_score = fuzzy_score(&track.album, query);
    let file_score = fuzzy_score(filename, query);

    [title_score, artist_score, album_score, file_score]
        .into_iter()
        .flatten()
        .max()
}

#[derive(Clone, Debug, Default)]
pub struct SearchState {
    pub is_open: bool,
    pub query: String,
    pub pool: Vec<Track>,
    pub results: Vec<Track>,
    pub selected_idx: usize,
}

impl SearchState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self, pool: Vec<Track>) {
        self.is_open = true;
        self.query.clear();
        self.pool = pool;
        self.selected_idx = 0;
        self.refresh_results();
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.query.clear();
        self.pool.clear();
        self.results.clear();
        self.selected_idx = 0;
    }

    pub fn type_char(&mut self, c: char) {
        self.query.push(c);
        self.selected_idx = 0;
        self.refresh_results();
    }

    pub fn backspace(&mut self) {
        self.query.pop();
        self.selected_idx = 0;
        self.refresh_results();
    }

    pub fn move_up(&mut self) {
        if self.selected_idx > 0 {
            self.selected_idx -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if !self.results.is_empty() && self.selected_idx + 1 < self.results.len() {
            self.selected_idx += 1;
        }
    }

    pub fn selected_track(&self) -> Option<&Track> {
        self.results.get(self.selected_idx)
    }

    pub fn refresh_results(&mut self) {
        if self.query.trim().is_empty() {
            self.results = self.pool.iter().take(50).cloned().collect();
            return;
        }

        let mut scored: Vec<(i64, Track)> = self
            .pool
            .iter()
            .filter_map(|t| score_track(t, &self.query).map(|s| (s, t.clone())))
            .collect();

        // Sort descending by score
        scored.sort_by(|a, b| b.0.cmp(&a.0));

        self.results = scored.into_iter().take(50).map(|(_, t)| t).collect();
        if self.selected_idx >= self.results.len() {
            self.selected_idx = self.results.len().saturating_sub(1);
        }
    }
}
