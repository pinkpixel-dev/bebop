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

        // Vertical split: Header (3 rows), Middle Area (min 8 rows), Controls (7 rows), Status (3 rows)
        let vert_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),      // Header / Nav
                Constraint::Min(8),         // Artwork + Visualizer
                Constraint::Length(7),      // Track title, timeline, controls
                Constraint::Length(3),      // Tech details, repeat, shuffle
            ])
            .split(area);

        let header = vert_chunks[0];
        let middle = vert_chunks[1];
        let player_controls = vert_chunks[2];
        let status = vert_chunks[3];

        // Middle horizontal split: Artwork on left (width ~24 cells), Visualizer on right
        let (artwork, visualizer) = if show_artwork && middle.width >= 50 {
            let art_width = (middle.height * 2).min(26).min(middle.width / 3);
            let horiz_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(art_width),
                    Constraint::Min(20),
                ])
                .split(middle);

            (Some(horiz_chunks[0]), horiz_chunks[1])
        } else {
            (None, middle)
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
