use std::fs;
use std::path::PathBuf;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Paragraph};
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

pub struct LibraryState {
    pub current_dir: PathBuf,
    pub folder_entries: Vec<PathBuf>,
    pub saved_playlists: Vec<PlaylistInfo>,
    pub selected_folder_idx: usize,
    pub track_entries: Vec<Track>,
    pub selected_track_idx: usize,
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
            track_entries: Vec::new(),
            selected_track_idx: 0,
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

        if self.selected_folder_idx >= self.folder_entries.len() + self.saved_playlists.len() {
            self.selected_folder_idx = 0;
        }
        if self.selected_track_idx >= self.track_entries.len() {
            self.selected_track_idx = 0;
        }
    }

    pub fn enter_selected_folder(&mut self) {
        if self.selected_folder_idx < self.folder_entries.len() {
            let target = &self.folder_entries[self.selected_folder_idx];
            if let Ok(canonical) = target.canonicalize() {
                self.current_dir = canonical;
                self.selected_folder_idx = 0;
                self.refresh();
            }
        } else {
            // Selected a saved playlist
            let playlist_idx = self.selected_folder_idx - self.folder_entries.len();
            if let Some(pl) = self.saved_playlists.get(playlist_idx) {
                if let Ok(tracks) = PlaylistManager::load_m3u(&pl.path) {
                    self.track_entries = tracks;
                    self.selected_track_idx = 0;
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
}

pub struct LibraryView;

impl LibraryView {
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        state: &LibraryState,
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
        let left_focused = state.focused_panel == LibraryPanel::Folders;
        let left_border_color = if left_focused { theme.border_focused } else { theme.border };
        let left_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(left_border_color))
            .title(Span::styled(" Folders & Playlists (← / h) ", Style::default().fg(if left_focused { theme.text } else { theme.text_dim })));

        let mut folder_items = Vec::new();
        for (i, p) in state.folder_entries.iter().enumerate() {
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("..");
            let is_parent = name == "..";
            let label = if is_parent { " ⮬ [Parent Directory]".to_string() } else { format!(" 📁 {}", name) };

            let style = if left_focused && state.selected_folder_idx == i {
                Style::default().fg(theme.accent).bg(theme.surface).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text)
            };
            folder_items.push(ListItem::new(label).style(style));
        }

        if !state.saved_playlists.is_empty() {
            folder_items.push(ListItem::new(" ── Saved Playlists ──").style(Style::default().fg(theme.text_dim)));
            for (i, pl) in state.saved_playlists.iter().enumerate() {
                let actual_idx = state.folder_entries.len() + i;
                let label = format!(" ★ {} ({} tracks)", pl.name, pl.track_count);
                let style = if left_focused && state.selected_folder_idx == actual_idx {
                    Style::default().fg(theme.accent).bg(theme.surface).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.visualizer_primary)
                };
                folder_items.push(ListItem::new(label).style(style));
            }
        }

        let folder_list = List::new(folder_items).block(left_block);
        frame.render_widget(folder_list, left_col);

        // Render Right Panel (Tracks)
        let right_focused = state.focused_panel == LibraryPanel::Tracks;
        let right_border_color = if right_focused { theme.border_focused } else { theme.border };
        let track_count_label = format!(" Tracks ({}) (→ / l) ", state.track_entries.len());
        let right_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(right_border_color))
            .title(Span::styled(track_count_label, Style::default().fg(if right_focused { theme.text } else { theme.text_dim })));

        let mut track_items = Vec::new();
        if state.track_entries.is_empty() {
            track_items.push(ListItem::new("  (No audio files found in this folder)").style(Style::default().fg(theme.text_dim)));
        } else {
            for (i, t) in state.track_entries.iter().enumerate() {
                let num_prefix = t.track_number.map(|n| format!("{:02}. ", n)).unwrap_or_else(|| format!("{:02}. ", i + 1));
                let dur = t.formatted_duration();
                let label = format!(" {}{:<40}  {:>8}", num_prefix, t.title, dur);

                let style = if right_focused && state.selected_track_idx == i {
                    Style::default().fg(theme.accent).bg(theme.surface).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text)
                };
                track_items.push(ListItem::new(label).style(style));
            }
        }

        let track_list = List::new(track_items).block(right_block);
        frame.render_widget(track_list, right_col);

        // 3. Footer instructions
        let tips = Line::from(vec![
            Span::styled("Enter: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("Play / Open Folder  ", Style::default().fg(theme.text_muted)),
            Span::styled("a: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("Add to Queue  ", Style::default().fg(theme.text_muted)),
            Span::styled("h/l or ←/→: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("Switch Panel  ", Style::default().fg(theme.text_muted)),
            Span::styled("1: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("Player View", Style::default().fg(theme.text_muted)),
        ]);
        frame.render_widget(Paragraph::new(tips).alignment(Alignment::Center), footer_area);
    }
}
