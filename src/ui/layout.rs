use ratatui::layout::{Constraint, Direction, Layout, Rect};

#[allow(dead_code)]
pub struct AppLayout {
    pub outer: Rect,
    pub header: Rect,
    pub middle: Rect,
    pub artwork: Option<Rect>,
    pub visualizer: Rect,
    pub player_controls: Rect,
    pub pet: Option<Rect>,
    pub status: Rect,
}

impl AppLayout {
    pub fn calculate(area: Rect, show_artwork: bool, show_pet: bool) -> Self {
        if area.width < 10 || area.height < 8 {
            return Self {
                outer: area,
                header: area,
                middle: area,
                artwork: None,
                visualizer: area,
                player_controls: area,
                pet: None,
                status: area,
            };
        }

        let deck_height = if area.height >= 30 { 9 } else { 8 };

        // Vertical split: Header (3 rows), Full-width Visualizer (min 6 rows), Bottom Deck (8-9 rows), Status (3 rows)
        let vert_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),               // Header / Nav
                Constraint::Min(6),                  // Full-width Visualizer
                Constraint::Length(deck_height),     // Bottom Deck (Artwork + Controls + Pet)
                Constraint::Length(3),               // Status Bar
            ])
            .split(area);

        let header = vert_chunks[0];
        let visualizer = vert_chunks[1];
        let middle = visualizer;
        let deck_area = vert_chunks[2];
        let status = vert_chunks[3];

        // Bottom Deck horizontal split: Artwork on left, Controls in middle, Pet on right
        let inner_h = deck_area.height.saturating_sub(2);
        let box_width = (inner_h * 2 + 2).min(deck_area.width / 3);

        // Terminal width below 55 collapses both side boxes to prioritize player controls
        let (has_art, has_pet) = if deck_area.width < 55 {
            (false, false)
        } else {
            match (show_artwork, show_pet) {
                (true, true) => {
                    if deck_area.width >= box_width * 2 + 35 {
                        (true, true)
                    } else {
                        (true, false)
                    }
                }
                (true, false) => (true, false),
                (false, true) => (false, true),
                (false, false) => (false, false),
            }
        };

        let (artwork, player_controls, pet) = match (has_art, has_pet) {
            (true, true) => {
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Length(box_width),
                        Constraint::Min(30),
                        Constraint::Length(box_width),
                    ])
                    .split(deck_area);
                (Some(chunks[0]), chunks[1], Some(chunks[2]))
            }
            (true, false) => {
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Length(box_width),
                        Constraint::Min(30),
                    ])
                    .split(deck_area);
                (Some(chunks[0]), chunks[1], None)
            }
            (false, true) => {
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Min(30),
                        Constraint::Length(box_width),
                    ])
                    .split(deck_area);
                (None, chunks[0], Some(chunks[1]))
            }
            (false, false) => (None, deck_area, None),
        };

        Self {
            outer: area,
            header,
            middle,
            artwork,
            visualizer,
            player_controls,
            pet,
            status,
        }
    }
}
