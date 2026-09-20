use std::fs;
use std::path::PathBuf;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
    ScrollbarState,
};
use ratatui::Frame;

use crate::library::{MetadataReader, Track};
use crate::player::{PlaylistInfo, PlaylistManager};
use crate::theme::Theme;
use crate::ui::player::{HitAction, HitZone};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryPanel {
    Folders,
    Tracks,
}

#[derive(Clone, Debug)]
pub struct LibraryState {
    pub current_dir: PathBuf,
    pub folder_entries: Vec<PathBuf>,
    pub saved_playlists: Vec<PlaylistInfo>,
    pub selected_folder_idx: usize,
    pub folder_scroll_offset: usize,
    pub track_entries: Vec<Track>,
    pub selected_track_idx: usize,
    pub track_scroll_offset: usize,
    pub focused_panel: LibraryPanel,
}

impl LibraryState {
    pub fn new() -> Self {
        let default_dir = dirs::audio_dir()
            .filter(|d| d.exists())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        let mut state = Self {
            current_dir: default_dir,
            folder_entries: Vec::new(),
            saved_playlists: Vec::new(),
            selected_folder_idx: 0,
            folder_scroll_offset: 0,
            track_entries: Vec::new(),
            selected_track_idx: 0,
            track_scroll_offset: 0,
            focused_panel: LibraryPanel::Folders,
        };

        state.refresh();
        state
    }

    pub fn refresh(&mut self) {
        // Scan current directory for subdirectories
        self.folder_entries.clear();
        self.track_entries.clear();

        // Add parent directory ".." if not root
        if self.current_dir.parent().is_some() {
            self.folder_entries.push(self.current_dir.join(".."));
        }

        if let Ok(entries) = fs::read_dir(&self.current_dir) {
            let mut dirs = Vec::new();
            let mut files = Vec::new();
            let valid_exts = ["mp3", "flac", "wav", "ogg", "m4a", "aac"];

            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    dirs.push(p);
                } else if let Some(ext) = p.extension().and_then(|s| s.to_str()).map(|s| s.to_lowercase()) {
                    if valid_exts.contains(&ext.as_str()) {
                        files.push(p);
                    }
                }
            }

            dirs.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
            files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

            self.folder_entries.extend(dirs);

            for f in files {
                let (track, _) = MetadataReader::read_track(&f);
                self.track_entries.push(track);
            }
        }

        self.saved_playlists = PlaylistManager::list_saved_playlists();

        let total_folders = self.folder_entries.len() + self.saved_playlists.len();
        if self.selected_folder_idx >= total_folders {
            self.selected_folder_idx = 0;
            self.folder_scroll_offset = 0;
        }
        if self.selected_track_idx >= self.track_entries.len() {
            self.selected_track_idx = 0;
            self.track_scroll_offset = 0;
        }
    }

    pub fn enter_selected_folder(&mut self) {
        if self.selected_folder_idx < self.folder_entries.len() {
            let target = &self.folder_entries[self.selected_folder_idx];
            if let Ok(canonical) = target.canonicalize() {
                self.current_dir = canonical;
                self.selected_folder_idx = 0;
                self.folder_scroll_offset = 0;
                self.selected_track_idx = 0;
                self.track_scroll_offset = 0;
                self.refresh();
            }
        } else {
            // Selected a saved playlist
            let playlist_idx = self.selected_folder_idx - self.folder_entries.len();
            if let Some(pl) = self.saved_playlists.get(playlist_idx) {
                if let Ok(tracks) = PlaylistManager::load_m3u(&pl.path) {
                    self.track_entries = tracks;
                    self.selected_track_idx = 0;
                    self.track_scroll_offset = 0;
                    self.focused_panel = LibraryPanel::Tracks;
                }
            }
        }
    }

    pub fn move_up(&mut self) {
        match self.focused_panel {
            LibraryPanel::Folders => {
                let total = self.folder_entries.len() + self.saved_playlists.len();
                if total > 0 && self.selected_folder_idx > 0 {
                    self.selected_folder_idx -= 1;
                }
            }
            LibraryPanel::Tracks => {
                if !self.track_entries.is_empty() && self.selected_track_idx > 0 {
                    self.selected_track_idx -= 1;
                }
            }
        }
    }

    pub fn move_down(&mut self) {
        match self.focused_panel {
            LibraryPanel::Folders => {
                let total = self.folder_entries.len() + self.saved_playlists.len();
                if total > 0 && self.selected_folder_idx + 1 < total {
                    self.selected_folder_idx += 1;
                }
            }
            LibraryPanel::Tracks => {
                if !self.track_entries.is_empty() && self.selected_track_idx + 1 < self.track_entries.len() {
                    self.selected_track_idx += 1;
                }
            }
        }
    }

    pub fn page_up(&mut self, page_size: usize) {
        match self.focused_panel {
            LibraryPanel::Folders => {
                self.selected_folder_idx = self.selected_folder_idx.saturating_sub(page_size);
            }
            LibraryPanel::Tracks => {
                self.selected_track_idx = self.selected_track_idx.saturating_sub(page_size);
            }
        }
    }

    pub fn page_down(&mut self, page_size: usize) {
        match self.focused_panel {
            LibraryPanel::Folders => {
                let total = self.folder_entries.len() + self.saved_playlists.len();
                if total > 0 {
                    self.selected_folder_idx = (self.selected_folder_idx + page_size).min(total - 1);
                }
            }
            LibraryPanel::Tracks => {
                if !self.track_entries.is_empty() {
                    self.selected_track_idx = (self.selected_track_idx + page_size).min(self.track_entries.len() - 1);
                }
            }
        }
    }

    pub fn jump_to_start(&mut self) {
        match self.focused_panel {
            LibraryPanel::Folders => self.selected_folder_idx = 0,
            LibraryPanel::Tracks => self.selected_track_idx = 0,
        }
    }

    pub fn jump_to_end(&mut self) {
        match self.focused_panel {
            LibraryPanel::Folders => {
                let total = self.folder_entries.len() + self.saved_playlists.len();
                if total > 0 {
                    self.selected_folder_idx = total - 1;
                }
            }
            LibraryPanel::Tracks => {
                if !self.track_entries.is_empty() {
                    self.selected_track_idx = self.track_entries.len() - 1;
                }
            }
        }
    }

    pub fn ensure_folders_visible(&mut self, visible_height: usize) {
        let total_items = self.folder_entries.len() + if !self.saved_playlists.is_empty() { self.saved_playlists.len() + 1 } else { 0 };
        if visible_height == 0 || total_items == 0 {
            self.folder_scroll_offset = 0;
            return;
        }
        let target_idx = if self.selected_folder_idx < self.folder_entries.len() {
            self.selected_folder_idx
        } else {
            self.selected_folder_idx + 1
        };
        if target_idx < self.folder_scroll_offset {
            self.folder_scroll_offset = target_idx;
        } else if target_idx >= self.folder_scroll_offset + visible_height {
            self.folder_scroll_offset = target_idx + 1 - visible_height;
        }
        let max_offset = total_items.saturating_sub(visible_height);
        if self.folder_scroll_offset > max_offset {
            self.folder_scroll_offset = max_offset;
        }
    }

    pub fn ensure_tracks_visible(&mut self, visible_height: usize) {
        let total = self.track_entries.len();
        if visible_height == 0 || total == 0 {
            self.track_scroll_offset = 0;
            return;
        }
        if self.selected_track_idx >= total {
            self.selected_track_idx = total.saturating_sub(1);
        }
        if self.selected_track_idx < self.track_scroll_offset {
            self.track_scroll_offset = self.selected_track_idx;
        } else if self.selected_track_idx >= self.track_scroll_offset + visible_height {
            self.track_scroll_offset = self.selected_track_idx + 1 - visible_height;
        }
        let max_offset = total.saturating_sub(visible_height);
        if self.track_scroll_offset > max_offset {
            self.track_scroll_offset = max_offset;
        }
    }
}

pub struct LibraryView;

impl LibraryView {
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        state: &mut LibraryState,
        theme: &Theme,
        hit_zones: &mut Vec<HitZone>,
    ) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.height < 6 {
            return;
        }

        // Layout: Header (3), Dual-column (min 4), Footer (2)
        let vert_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(4),
                Constraint::Length(2),
            ])
            .split(inner);

        let header_area = vert_chunks[0];
        let content_area = vert_chunks[1];
        let footer_area = vert_chunks[2];

        // 1. Header
        let header_title = Line::from(vec![
            Span::styled(" ♪ Bebop  ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("[Library Browser]  ", Style::default().fg(theme.text).add_modifier(Modifier::BOLD)),
            Span::styled(format!("📁 {}", state.current_dir.display()), Style::default().fg(theme.text_dim)),
        ]);
        let nav_tabs = Line::from(vec![
            Span::styled(" Player  ", Style::default().fg(theme.text_muted)),
            Span::styled("[Library] ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(" Queue  ", Style::default().fg(theme.text_muted)),
            Span::styled(" Lyrics  ", Style::default().fg(theme.text_muted)),
            Span::styled(" ? Help ", Style::default().fg(theme.text_dim)),
        ]);
        frame.render_widget(Paragraph::new(header_title).alignment(Alignment::Left), header_area);
        frame.render_widget(Paragraph::new(nav_tabs).alignment(Alignment::Right), header_area);

        // Record header tab hit zones
        if header_area.width > 42 {
            let right_start = header_area.right().saturating_sub(40);
            hit_zones.push(HitZone {
                rect: Rect::new(right_start, header_area.top(), 9, 1),
                action: HitAction::TabPlayer,
            });
            hit_zones.push(HitZone {
                rect: Rect::new(right_start + 9, header_area.top(), 10, 1),
                action: HitAction::TabLibrary,
            });
            hit_zones.push(HitZone {
                rect: Rect::new(right_start + 19, header_area.top(), 7, 1),
                action: HitAction::TabQueue,
            });
            hit_zones.push(HitZone {
                rect: Rect::new(right_start + 26, header_area.top(), 8, 1),
                action: HitAction::TabLyrics,
            });
            hit_zones.push(HitZone {
                rect: Rect::new(right_start + 34, header_area.top(), 7, 1),
                action: HitAction::TabHelp,
            });
        }

        // 2. Dual-column content split: Folders (35%) | Tracks (65%)
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(35),
                Constraint::Percentage(65),
            ])
            .split(content_area);

        let left_col = cols[0];
        let right_col = cols[1];

        // Render Left Panel (Folders & Playlists)
        let left_visible_height = left_col.height.saturating_sub(2) as usize;
        state.ensure_folders_visible(left_visible_height);

        let left_focused = state.focused_panel == LibraryPanel::Folders;
        let left_border_color = if left_focused { theme.border_focused } else { theme.border };
        let left_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(left_border_color))
            .title(Span::styled(" Folders & Playlists (← / h) ", Style::default().fg(if left_focused { theme.text } else { theme.text_dim })));

        // Build all visual items for folder panel: (ListItem, Option<HitAction>)
        let mut all_folder_items: Vec<(ListItem, Option<HitAction>)> = Vec::new();
        for (i, p) in state.folder_entries.iter().enumerate() {
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("..");
            let is_parent = name == "..";
            let label = if is_parent { " ⮬ [Parent Directory]".to_string() } else { format!(" 📁 {}", name) };

            let style = if left_focused && state.selected_folder_idx == i {
                Style::default().fg(theme.accent).bg(theme.surface).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text)
            };
            all_folder_items.push((ListItem::new(label).style(style), Some(HitAction::LibraryFolderSelect(i))));
        }

        if !state.saved_playlists.is_empty() {
            all_folder_items.push((ListItem::new(" ── Saved Playlists ──").style(Style::default().fg(theme.text_dim)), None));
            for (i, pl) in state.saved_playlists.iter().enumerate() {
                let actual_idx = state.folder_entries.len() + i;
                let label = format!(" ★ {} ({} tracks)", pl.name, pl.track_count);
                let style = if left_focused && state.selected_folder_idx == actual_idx {
                    Style::default().fg(theme.accent).bg(theme.surface).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.visualizer_primary)
                };
                all_folder_items.push((ListItem::new(label).style(style), Some(HitAction::LibraryFolderSelect(actual_idx))));
            }
        }

        let total_folder_items = all_folder_items.len();
        let mut visible_folder_widgets = Vec::new();
        let visible_slice = all_folder_items
            .into_iter()
            .enumerate()
            .skip(state.folder_scroll_offset)
            .take(left_visible_height);

        for (row_idx, (item, action)) in visible_slice {
            visible_folder_widgets.push(item);

            if let Some(act) = action {
                let row_y = left_col.top() + 1 + (row_idx - state.folder_scroll_offset) as u16;
                if row_y < left_col.bottom().saturating_sub(1) {
                    hit_zones.push(HitZone {
                        rect: Rect::new(left_col.left() + 1, row_y, left_col.width.saturating_sub(2), 1),
                        action: act,
                    });
                }
            }
        }

        let folder_list = List::new(visible_folder_widgets).block(left_block);
        frame.render_widget(folder_list, left_col);

        // Render scrollbar on left panel if it exceeds visible height
        if total_folder_items > left_visible_height {
            let target_pos = if state.selected_folder_idx < state.folder_entries.len() {
                state.selected_folder_idx
            } else {
                state.selected_folder_idx + 1
            };
            let mut scrollbar_state = ScrollbarState::new(total_folder_items)
                .position(target_pos)
                .viewport_content_length(left_visible_height);
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .end_symbol(None)
                    .track_symbol(Some("│"))
                    .thumb_symbol("█")
                    .style(Style::default().fg(theme.text_dim)),
                left_col,
                &mut scrollbar_state,
            );
        }

        // Render Right Panel (Tracks)
        let right_visible_height = right_col.height.saturating_sub(2) as usize;
        state.ensure_tracks_visible(right_visible_height);

        let right_focused = state.focused_panel == LibraryPanel::Tracks;
        let right_border_color = if right_focused { theme.border_focused } else { theme.border };
        let track_count_label = if state.track_entries.len() > right_visible_height && !state.track_entries.is_empty() {
            format!(" Tracks ({}) [{}/{}] (→ / l) ", state.track_entries.len(), state.selected_track_idx + 1, state.track_entries.len())
        } else {
            format!(" Tracks ({}) (→ / l) ", state.track_entries.len())
        };
        let right_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(right_border_color))
            .title(Span::styled(track_count_label, Style::default().fg(if right_focused { theme.text } else { theme.text_dim })));

        let mut track_items = Vec::new();
        if state.track_entries.is_empty() {
            track_items.push(ListItem::new("  (No audio files found in this folder)").style(Style::default().fg(theme.text_dim)));
        } else {
            let visible_tracks = state
                .track_entries
                .iter()
                .enumerate()
                .skip(state.track_scroll_offset)
                .take(right_visible_height);

            for (i, t) in visible_tracks {
                let num_prefix = t.track_number.map(|n| format!("{:02}. ", n)).unwrap_or_else(|| format!("{:02}. ", i + 1));
                let dur = t.formatted_duration();
                let label = format!(" {}{:<40}  {:>8}", num_prefix, t.title, dur);

                let style = if right_focused && state.selected_track_idx == i {
                    Style::default().fg(theme.accent).bg(theme.surface).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text)
                };
                track_items.push(ListItem::new(label).style(style));

                // Hit zone for tap / click selection
                let row_offset = (i - state.track_scroll_offset) as u16;
                let row_y = right_col.top() + 1 + row_offset;
                if row_y < right_col.bottom().saturating_sub(1) {
                    hit_zones.push(HitZone {
                        rect: Rect::new(right_col.left() + 1, row_y, right_col.width.saturating_sub(2), 1),
                        action: HitAction::LibraryTrackSelect(i),
                    });
                }
            }
        }

        let track_list = List::new(track_items).block(right_block);
        frame.render_widget(track_list, right_col);

        // Render scrollbar on right panel if tracks exceed visible height
        if state.track_entries.len() > right_visible_height {
            let mut scrollbar_state = ScrollbarState::new(state.track_entries.len())
                .position(state.selected_track_idx)
                .viewport_content_length(right_visible_height);
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .end_symbol(None)
                    .track_symbol(Some("│"))
                    .thumb_symbol("█")
                    .style(Style::default().fg(theme.text_dim)),
                right_col,
                &mut scrollbar_state,
            );
        }

        // 3. Footer instructions
        let tips = Line::from(vec![
            Span::styled("Enter: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("Play / Open  ", Style::default().fg(theme.text_muted)),
            Span::styled("a: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("Add to Queue  ", Style::default().fg(theme.text_muted)),
            Span::styled("h/l or ←/→: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("Switch Panel  ", Style::default().fg(theme.text_muted)),
            Span::styled("PgUp/PgDn: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("Scroll  ", Style::default().fg(theme.text_muted)),
            Span::styled("1: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("Player View", Style::default().fg(theme.text_muted)),
        ]);
        frame.render_widget(Paragraph::new(tips).alignment(Alignment::Center), footer_area);
    }
}
