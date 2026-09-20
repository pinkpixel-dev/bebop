use std::io::Cursor;
use std::path::Path;
use std::time::Duration;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::backend::Backend;
use ratatui::Terminal;

use crate::audio::{AudioEngine, PlaybackState};
use crate::library::{MetadataReader, Track};
use crate::player::{PlayerState, PlaylistManager, Queue, RepeatMode};
use crate::terminal::{KittyRenderer, TerminalDetector, TerminalGraphics};
use crate::theme::Theme;
use crate::ui::player::{HitAction, HitZone, PlayerView};
use crate::ui::{AppLayout, FullscreenView, HelpOverlay, LibraryPanel, LibraryState, LibraryView, QueueState, QueueView};
use crate::visualizers::bars::BarsVisualizer;
use crate::visualizers::waveform::WaveformVisualizer;
use crate::visualizers::{Visualizer, VisualizerKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Player,
    Library,
    Queue,
}

pub struct App {
    pub player: PlayerState,
    pub queue: Queue,
    pub audio_engine: AudioEngine,
    pub theme: Theme,
    pub active_view: View,
    pub library_state: LibraryState,
    pub queue_state: QueueState,
    pub visualizer_kind: VisualizerKind,
    pub visualizer: Box<dyn Visualizer>,
    pub fullscreen_visualizer: bool,
    pub show_artwork: bool,
    pub show_help: bool,
    pub should_quit: bool,
    pub terminal_graphics: TerminalGraphics,
    pub artwork_png: Option<Vec<u8>>,
    hit_zones: Vec<HitZone>,
    last_art_rendered: Option<(u16, u16, u16, u16)>,
}

impl App {
    pub fn new() -> Result<Self, anyhow::Error> {
        let audio_engine = AudioEngine::new()?;
        let graphics = TerminalDetector::detect();
        let library_state = LibraryState::new();
        let queue_state = QueueState::new();

        Ok(Self {
            player: PlayerState::default(),
            queue: Queue::new(),
            audio_engine,
            theme: Theme::default(),
            active_view: View::Player,
            library_state,
            queue_state,
            visualizer_kind: VisualizerKind::Bars,
            visualizer: Box::new(BarsVisualizer::default()),
            fullscreen_visualizer: false,
            show_artwork: true,
            show_help: false,
            should_quit: false,
            terminal_graphics: graphics,
            artwork_png: None,
            hit_zones: Vec::new(),
            last_art_rendered: None,
        })
    }

    /// Open and play a target path (file, directory, or .m3u playlist).
    pub fn open_target<P: AsRef<Path>>(&mut self, path: P) -> Result<(), anyhow::Error> {
        let p = path.as_ref();
        if p.is_dir() {
            self.load_directory(p)
        } else if let Some(ext) = p.extension().and_then(|s| s.to_str()).map(|s| s.to_lowercase()) {
            if ext == "m3u" || ext == "m3u8" {
                self.load_playlist_file(p)
            } else {
                self.play_track_file(p)
            }
        } else {
            self.play_track_file(p)
        }
    }

    /// Load tracks from an M3U playlist file and begin playback.
    pub fn load_playlist_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), anyhow::Error> {
        let tracks = PlaylistManager::load_m3u(path.as_ref())?;
        if let Some(first) = tracks.first().cloned() {
            let track_path = first.path.clone();
            self.queue.set_tracks(tracks, 0);
            self.play_track_file(track_path)?;
        }
        Ok(())
    }

    /// Load and play a specific track file.
    pub fn play_track_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), anyhow::Error> {
        let p = path.as_ref();
        let (mut track, art_data) = MetadataReader::read_track(p);

        self.audio_engine.play_file(p)?;
        track.duration = self.audio_engine.duration();

        // Process artwork image
        self.artwork_png = None;
        self.last_art_rendered = None;
        let _ = KittyRenderer::clear_all();

        if let Some(bytes) = art_data {
            if let Ok(img) = image::load_from_memory(&bytes) {
                let resized = img.resize_exact(300, 300, image::imageops::FilterType::Triangle);
                let mut png_buf = Cursor::new(Vec::new());
                if resized.write_to(&mut png_buf, image::ImageFormat::Png).is_ok() {
                    self.artwork_png = Some(png_buf.into_inner());
                }
            }
        }

        self.player.current_track = Some(track.clone());
        self.player.playback_state = PlaybackState::Playing;
        self.player.position = Duration::ZERO;
        self.player.duration = self.audio_engine.duration();

        // If track is in queue, sync current_index; otherwise add it
        if let Some(idx) = self.queue.tracks.iter().position(|t| t.path == track.path) {
            self.queue.current_index = Some(idx);
        } else {
            self.queue.add_track(track);
        }

        Ok(())
    }

    /// Recursively scan a folder and queue playable audio tracks.
    pub fn load_directory<P: AsRef<Path>>(&mut self, dir: P) -> Result<(), anyhow::Error> {
        let mut tracks = Vec::new();
        self.scan_folder(dir.as_ref(), &mut tracks);

        if let Some(first) = tracks.first().cloned() {
            let path = first.path.clone();
            self.queue.set_tracks(tracks, 0);
            self.play_track_file(path)?;
        }

        Ok(())
    }

    fn scan_folder(&self, dir: &Path, tracks: &mut Vec<Track>) {
        let valid_extensions = ["mp3", "flac", "wav", "ogg", "m4a", "aac"];
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    self.scan_folder(&p, tracks);
                } else if let Some(ext) = p.extension().and_then(|s| s.to_str()).map(|s| s.to_lowercase()) {
                    if valid_extensions.contains(&ext.as_str()) {
                        let (track, _) = MetadataReader::read_track(&p);
                        tracks.push(track);
                    }
                }
            }
        }
    }

    pub fn on_tick(&mut self) {
        self.player.playback_state = self.audio_engine.state();
        self.player.position = self.audio_engine.current_position();
        if self.player.duration == Duration::ZERO {
            self.player.duration = self.audio_engine.duration();
        }
        self.player.volume = self.audio_engine.volume();
        self.player.muted = self.audio_engine.is_muted();

        // Check if track reached end
        if self.audio_engine.is_finished() {
            self.next_track();
        }
    }

    pub fn next_track(&mut self) {
        let repeat_all = self.player.repeat == RepeatMode::All;
        if self.player.repeat == RepeatMode::One {
            if let Some(cur) = self.queue.current_track().cloned() {
                let _ = self.play_track_file(&cur.path);
                return;
            }
        }

        if let Some(next) = self.queue.advance_next(repeat_all).cloned() {
            let _ = self.play_track_file(&next.path);
        } else {
            self.audio_engine.stop();
            self.player.playback_state = PlaybackState::Stopped;
        }
    }

    pub fn prev_track(&mut self) {
        if self.player.position > Duration::from_secs(3) {
            self.audio_engine.seek_to(Duration::ZERO);
            return;
        }

        if let Some(prev) = self.queue.advance_prev().cloned() {
            let _ = self.play_track_file(&prev.path);
        }
    }

    pub fn toggle_play_pause(&mut self) {
        let new_state = self.audio_engine.toggle_play_pause();
        self.player.playback_state = new_state;
    }

    pub fn cycle_visualizer(&mut self) {
        self.visualizer_kind = self.visualizer_kind.next();
        self.visualizer = match self.visualizer_kind {
            VisualizerKind::Bars => Box::new(BarsVisualizer::default()),
            VisualizerKind::Waveform => Box::new(WaveformVisualizer::default()),
        };
    }

    pub fn cycle_theme(&mut self) {
        self.theme = self.theme.cycle_next();
    }

    pub fn handle_action(&mut self, action: HitAction) {
        match action {
            HitAction::TabPlayer => {
                self.active_view = View::Player;
                self.fullscreen_visualizer = false;
                self.show_help = false;
            }
            HitAction::TabLibrary => {
                self.active_view = View::Library;
                self.fullscreen_visualizer = false;
                self.show_help = false;
                let _ = KittyRenderer::clear_all();
                self.last_art_rendered = None;
            }
            HitAction::TabQueue => {
                self.active_view = View::Queue;
                self.fullscreen_visualizer = false;
                self.show_help = false;
                let _ = KittyRenderer::clear_all();
                self.last_art_rendered = None;
            }
            HitAction::TabHelp => {
                self.show_help = !self.show_help;
            }
            HitAction::VisualizerCycle => {
                self.cycle_visualizer();
            }
            HitAction::FullscreenToggle => {
                self.fullscreen_visualizer = !self.fullscreen_visualizer;
                if self.fullscreen_visualizer {
                    let _ = KittyRenderer::clear_all();
                    self.last_art_rendered = None;
                }
            }
            HitAction::SeekRatio(ratio) => {
                let total_ms = self.player.duration.as_millis() as f32;
                let target_ms = (ratio * total_ms) as u64;
                self.audio_engine.seek_to(Duration::from_millis(target_ms));
            }
            HitAction::VolumeRatio(ratio) => {
                self.audio_engine.set_volume(ratio);
                self.player.volume = ratio;
            }
            HitAction::PreviousTrack => self.prev_track(),
            HitAction::PlayPause => self.toggle_play_pause(),
            HitAction::NextTrack => self.next_track(),
            HitAction::ToggleRepeat => {
                self.player.repeat = self.player.repeat.cycle();
            }
            HitAction::ToggleShuffle => {
                self.player.shuffle = !self.player.shuffle;
            }
        }
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        if self.show_help {
            if key.code == KeyCode::Esc || key.code == KeyCode::Char('?') || key.code == KeyCode::Char('q') {
                self.show_help = false;
            }
            return;
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }

        // Global view switching
        match key.code {
            KeyCode::Char('1') => {
                self.active_view = View::Player;
                return;
            }
            KeyCode::Char('2') => {
                self.active_view = View::Library;
                let _ = KittyRenderer::clear_all();
                self.last_art_rendered = None;
                return;
            }
            KeyCode::Char('3') => {
                self.active_view = View::Queue;
                let _ = KittyRenderer::clear_all();
                self.last_art_rendered = None;
                return;
            }
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
                return;
            }
            _ => {}
        }

        // View-specific key navigation
        match self.active_view {
            View::Library => {
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => self.library_state.move_up(),
                    KeyCode::Down | KeyCode::Char('j') => self.library_state.move_down(),
                    KeyCode::Left | KeyCode::Char('h') => self.library_state.focused_panel = LibraryPanel::Folders,
                    KeyCode::Right | KeyCode::Char('l') => self.library_state.focused_panel = LibraryPanel::Tracks,
                    KeyCode::Enter => {
                        if self.library_state.focused_panel == LibraryPanel::Folders {
                            self.library_state.enter_selected_folder();
                        } else if !self.library_state.track_entries.is_empty() {
                            let idx = self.library_state.selected_track_idx;
                            let tracks = self.library_state.track_entries.clone();
                            if let Some(track) = tracks.get(idx) {
                                let p = track.path.clone();
                                self.queue.set_tracks(tracks, idx);
                                let _ = self.play_track_file(p);
                                self.active_view = View::Player;
                            }
                        }
                    }
                    KeyCode::Char('a') => {
                        if self.library_state.focused_panel == LibraryPanel::Folders {
                            // Enqueue all tracks in current folder
                            for t in &self.library_state.track_entries {
                                self.queue.add_track(t.clone());
                            }
                        } else if let Some(t) = self.library_state.track_entries.get(self.library_state.selected_track_idx) {
                            self.queue.add_track(t.clone());
                        }
                    }
                    KeyCode::Char('p') => {
                        // Save current queue as a playlist
                        if !self.queue.tracks.is_empty() {
                            let name = format!("Playlist_{}", self.library_state.saved_playlists.len() + 1);
                            let _ = PlaylistManager::save_named_playlist(&name, &self.queue.tracks);
                            self.library_state.refresh();
                        }
                    }
                    KeyCode::Char('q') | KeyCode::Esc => self.active_view = View::Player,
                    _ => {}
                }
                return;
            }
            View::Queue => {
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => self.queue_state.move_up(),
                    KeyCode::Down | KeyCode::Char('j') => self.queue_state.move_down(self.queue.tracks.len()),
                    KeyCode::Enter => {
                        if let Some(t) = self.queue.jump_to(self.queue_state.selected_idx).cloned() {
                            let _ = self.play_track_file(t.path);
                            self.active_view = View::Player;
                        }
                    }
                    KeyCode::Char('d') | KeyCode::Delete => {
                        self.queue.remove_at(self.queue_state.selected_idx);
                        if self.queue_state.selected_idx >= self.queue.tracks.len() && !self.queue.tracks.is_empty() {
                            self.queue_state.selected_idx = self.queue.tracks.len() - 1;
                        }
                    }
                    KeyCode::Char('c') => {
                        self.queue.clear();
                        self.audio_engine.stop();
                        self.player.playback_state = PlaybackState::Stopped;
                        self.player.current_track = None;
                    }
                    KeyCode::Char('s') => self.player.shuffle = !self.player.shuffle,
                    KeyCode::Char('q') | KeyCode::Esc => self.active_view = View::Player,
                    _ => {}
                }
                return;
            }
            View::Player => {}
        }

        // Global playback keys in Player view
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Esc => {
                if self.fullscreen_visualizer {
                    self.fullscreen_visualizer = false;
                }
            }
            KeyCode::Char(' ') => self.toggle_play_pause(),
            KeyCode::Char('n') => self.next_track(),
            KeyCode::Char('p') => self.prev_track(),
            KeyCode::Right => self.audio_engine.seek_relative(5),
            KeyCode::Left => self.audio_engine.seek_relative(-5),
            KeyCode::Up => {
                let _ = self.audio_engine.adjust_volume(0.05);
            }
            KeyCode::Down => {
                let _ = self.audio_engine.adjust_volume(-0.05);
            }
            KeyCode::Char('m') => {
                let _ = self.audio_engine.toggle_mute();
            }
            KeyCode::Char('v') => self.cycle_visualizer(),
            KeyCode::Char('f') => {
                self.fullscreen_visualizer = !self.fullscreen_visualizer;
                if self.fullscreen_visualizer {
                    let _ = KittyRenderer::clear_all();
                    self.last_art_rendered = None;
                }
            }
            KeyCode::Char('t') => self.cycle_theme(),
            KeyCode::Char('a') => {
                self.show_artwork = !self.show_artwork;
                if !self.show_artwork {
                    let _ = KittyRenderer::clear_all();
                    self.last_art_rendered = None;
                }
            }
            KeyCode::Char('r') => self.player.repeat = self.player.repeat.cycle(),
            KeyCode::Char('s') => self.player.shuffle = !self.player.shuffle,
            _ => {}
        }
    }

    pub fn on_mouse(&mut self, mouse: MouseEvent) {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let col = mouse.column;
                let row = mouse.row;

                for zone in self.hit_zones.clone() {
                    let r = zone.rect;
                    if col >= r.left() && col < r.right() && row >= r.top() && row < r.bottom() {
                        let action = match zone.action {
                            HitAction::SeekRatio(_) => {
                                let ratio = ((col.saturating_sub(r.left())) as f32 / r.width as f32).clamp(0.0, 1.0);
                                HitAction::SeekRatio(ratio)
                            }
                            HitAction::VolumeRatio(_) => {
                                let ratio = ((col.saturating_sub(r.left())) as f32 / r.width as f32).clamp(0.0, 1.0);
                                HitAction::VolumeRatio(ratio)
                            }
                            other => other,
                        };
                        self.handle_action(action);
                        break;
                    }
                }
            }
            MouseEventKind::ScrollUp => {
                let _ = self.audio_engine.adjust_volume(0.05);
            }
            MouseEventKind::ScrollDown => {
                let _ = self.audio_engine.adjust_volume(-0.05);
            }
            _ => {}
        }
    }

    pub fn draw<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<(), anyhow::Error> {
        self.hit_zones.clear();
        let audio_frame = self.audio_engine.get_audio_frame(64);
        let mut art_box_rect: Option<(u16, u16, u16, u16)> = None;

        let active_view = self.active_view;
        let is_fullscreen = self.fullscreen_visualizer;
        let show_art = self.show_artwork;

        terminal.draw(|frame| {
            let area = frame.area();

            match active_view {
                View::Library => {
                    LibraryView::render(frame, area, &self.library_state, &self.theme, &mut self.hit_zones);
                }
                View::Queue => {
                    QueueView::render(frame, area, &self.queue, &self.queue_state, &self.theme, &mut self.hit_zones);
                }
                View::Player => {
                    if is_fullscreen {
                        FullscreenView::render(
                            frame,
                            area,
                            &self.player,
                            &mut self.visualizer,
                            &audio_frame,
                            &self.theme,
                            &mut self.hit_zones,
                        );
                    } else {
                        let layout = AppLayout::calculate(area, show_art);

                        if let Some(art) = layout.artwork {
                            art_box_rect = Some((art.x + 1, art.y + 1, art.width.saturating_sub(2), art.height.saturating_sub(2)));
                        }

                        PlayerView::render(
                            frame,
                            &layout,
                            &self.player,
                            &mut self.visualizer,
                            &audio_frame,
                            &self.theme,
                            &mut self.hit_zones,
                        );
                    }
                }
            }

            if self.show_help {
                HelpOverlay::render(frame, area, &self.theme);
            }
        })?;

        // Render Kitty graphics album artwork only when on Player view
        if self.active_view == View::Player && !self.fullscreen_visualizer && self.show_artwork && self.terminal_graphics == TerminalGraphics::Kitty {
            if let (Some(rect), Some(png)) = (art_box_rect, &self.artwork_png) {
                if self.last_art_rendered != Some(rect) {
                    let _ = KittyRenderer::render_png(png, rect.0, rect.1, rect.2, rect.3);
                    self.last_art_rendered = Some(rect);
                }
            }
        }

        Ok(())
    }
}
