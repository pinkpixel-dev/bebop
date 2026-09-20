use ratatui::layout::{Constraint, Direction, Layout, Rect};

#[allow(dead_code)]
pub struct AppLayout {
    pub outer: Rect,
    pub header: Rect,
    pub middle: Rect,
    pub artwork: Option<Rect>,
    pub visualizer: Rect,
    pub player_controls: Rect,
    pub status: Rect,
}

impl AppLayout {
    pub fn calculate(area: Rect, show_artwork: bool) -> Self {
        if area.width < 10 || area.height < 8 {
            return Self {
                outer: area,
                header: area,
                middle: area,
                artwork: None,
                visualizer: area,
                player_controls: area,
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
                Constraint::Length(deck_height),     // Bottom Deck (Artwork + Controls)
                Constraint::Length(3),               // Status Bar
            ])
            .split(area);

        let header = vert_chunks[0];
        let visualizer = vert_chunks[1];
        let middle = visualizer;
        let deck_area = vert_chunks[2];
        let status = vert_chunks[3];

        // Bottom Deck horizontal split: Artwork on left (visually square), Controls on right
        let (artwork, player_controls) = if show_artwork && deck_area.width >= 55 {
            // In standard terminal fonts, character cell width is ~half cell height (1:2 aspect ratio).
            // To produce a visually square artwork box on screen:
            // inner_height = deck_height - 2
            // inner_width = inner_height * 2
            // box_width = inner_width + 2
            let inner_h = deck_area.height.saturating_sub(2);
            let art_width = (inner_h * 2 + 2).min(deck_area.width / 3);

            let deck_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(art_width),
                    Constraint::Min(30),
                ])
                .split(deck_area);

            (Some(deck_chunks[0]), deck_chunks[1])
        } else {
            (None, deck_area)
        };

        Self {
            outer: area,
            header,
            middle,
            artwork,
            visualizer,
            player_controls,
            status,
        }
    }
}
