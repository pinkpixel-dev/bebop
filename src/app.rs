use std::io::Cursor;
use std::path::Path;
use std::time::Duration;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::backend::Backend;
use ratatui::Terminal;

use crate::audio::{AudioEngine, PlaybackState};
use crate::config::AppConfig;
use crate::library::{MetadataReader, SearchState, Track};
use crate::lyrics::{Lyrics, LyricsState};
use crate::notifications::NotificationManager;
use crate::player::{PlayerState, PlaylistManager, Queue, RepeatMode};
use crate::terminal::{KittyRenderer, TerminalDetector, TerminalGraphics};
use crate::theme::{extract_palette, ExtractedPalette, Theme};
use crate::ui::player::{HitAction, HitZone, PlayerView};
use crate::ui::{AppLayout, FullscreenView, HelpOverlay, LibraryPanel, LibraryState, LibraryView, LyricsView, QueueState, QueueView, SearchOverlay};
use crate::visualizers::bars::BarsVisualizer;
use crate::visualizers::mirrored::MirroredBarsVisualizer;
use crate::visualizers::particles::ParticlesVisualizer;
use crate::visualizers::vu::VuMeterVisualizer;
use crate::visualizers::waterfall::WaterfallVisualizer;
use crate::visualizers::waveform::WaveformVisualizer;
use crate::visualizers::{Visualizer, VisualizerKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Player,
    Library,
    Queue,
    Lyrics,
}

pub struct App {
    pub player: PlayerState,
    pub queue: Queue,
    pub audio_engine: AudioEngine,
    pub theme: Theme,
    pub active_view: View,
    pub library_state: LibraryState,
    pub queue_state: QueueState,
    pub lyrics_state: LyricsState,
    pub visualizer_kind: VisualizerKind,
    pub visualizer: Box<dyn Visualizer>,
    pub fullscreen_visualizer: bool,
    pub show_artwork: bool,
    pub show_help: bool,
    pub notifications_enabled: bool,
    pub search_state: SearchState,
    pub should_quit: bool,
    pub terminal_graphics: TerminalGraphics,
    pub artwork_png: Option<Vec<u8>>,
    pub current_palette: Option<ExtractedPalette>,
    hit_zones: Vec<HitZone>,
    last_art_rendered: Option<(u16, u16, u16, u16)>,
    pub config: AppConfig,
}

impl App {
    pub fn new() -> Result<Self, anyhow::Error> {
        let config = AppConfig::load();
        let audio_engine = AudioEngine::new()?;
        audio_engine.set_volume(config.player.volume);

        let theme = Theme::from_name(&config.ui.theme);
        let visualizer_kind = match config.ui.visualizer.to_lowercase().as_str() {
            "mirrored" | "mirrored bars" | "mirror" => VisualizerKind::Mirrored,
            "waveform" | "wave" => VisualizerKind::Waveform,
            "vu" | "vumeter" | "vu meter" | "stereo vu" => VisualizerKind::VuMeter,
            "waterfall" | "spectrogram" => VisualizerKind::Waterfall,
            "particles" | "particle" => VisualizerKind::Particles,
            _ => VisualizerKind::Bars,
        };
        let visualizer: Box<dyn Visualizer> = match visualizer_kind {
            VisualizerKind::Bars => Box::new(BarsVisualizer::default()),
            VisualizerKind::Mirrored => Box::new(MirroredBarsVisualizer::default()),
            VisualizerKind::Waveform => Box::new(WaveformVisualizer::default()),
            VisualizerKind::VuMeter => Box::new(VuMeterVisualizer::default()),
            VisualizerKind::Waterfall => Box::new(WaterfallVisualizer::default()),
            VisualizerKind::Particles => Box::new(ParticlesVisualizer::default()),
        };

        let repeat = match config.player.repeat.to_lowercase().as_str() {
            "one" | "track" => RepeatMode::One,
            "off" => RepeatMode::Off,
            _ => RepeatMode::All,
        };

        let mut player = PlayerState::default();
        player.volume = config.player.volume;
        player.repeat = repeat;
        player.shuffle = config.player.shuffle;

        let graphics = TerminalDetector::detect();
        let library_state = LibraryState::new();
        let queue_state = QueueState::new();
        let lyrics_state = LyricsState::new();

        let mut queue = Queue::new();
        queue.set_shuffle(config.player.shuffle);

        Ok(Self {
            player,
            queue,
            audio_engine,
            theme,
            active_view: View::Player,
            library_state,
            queue_state,
            lyrics_state,
            visualizer_kind,
            visualizer,
            fullscreen_visualizer: false,
            show_artwork: config.ui.artwork,
            show_help: false,
            notifications_enabled: config.ui.notifications,
            search_state: SearchState::new(),
            should_quit: false,
            terminal_graphics: graphics,
            artwork_png: None,
            current_palette: None,
            hit_zones: Vec::new(),
            last_art_rendered: None,
            config,
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
        self.current_palette = None;
        self.last_art_rendered = None;
        let _ = KittyRenderer::clear_all();

        if let Some(bytes) = art_data {
            if let Ok(img) = image::load_from_memory(&bytes) {
                let palette = extract_palette(&img);
                self.current_palette = Some(palette);

                let resized = img.resize_exact(300, 300, image::imageops::FilterType::Triangle);
                let mut png_buf = Cursor::new(Vec::new());
                if resized.write_to(&mut png_buf, image::ImageFormat::Png).is_ok() {
                    self.artwork_png = Some(png_buf.into_inner());
                }
            }
        }

        // If currently in Album theme, update theme colors dynamically from the new track's artwork
        if self.theme.name == "Album" {
            self.theme = Theme::album(self.current_palette);
        }

        self.player.current_track = Some(track.clone());
        self.player.playback_state = PlaybackState::Playing;
        self.player.position = Duration::ZERO;
        self.player.duration = self.audio_engine.duration();

        // Load synchronized lyrics if available
        let lrc = Lyrics::load_for_track(p);
        self.lyrics_state.set_lyrics(lrc);

        // Send desktop notification if enabled
        if self.notifications_enabled {
            NotificationManager::send_track_notification(&track);
        }

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

    pub fn toggle_shuffle(&mut self) {
        self.player.shuffle = !self.player.shuffle;
        self.queue.set_shuffle(self.player.shuffle);
    }

    pub fn cycle_visualizer(&mut self) {
        self.visualizer_kind = self.visualizer_kind.next();
        self.visualizer = match self.visualizer_kind {
            VisualizerKind::Bars => Box::new(BarsVisualizer::default()),
            VisualizerKind::Mirrored => Box::new(MirroredBarsVisualizer::default()),
            VisualizerKind::Waveform => Box::new(WaveformVisualizer::default()),
            VisualizerKind::VuMeter => Box::new(VuMeterVisualizer::default()),
            VisualizerKind::Waterfall => Box::new(WaterfallVisualizer::default()),
            VisualizerKind::Particles => Box::new(ParticlesVisualizer::default()),
        };
    }

    pub fn cycle_theme(&mut self) {
        self.theme = self.theme.cycle_next_with_palette(self.current_palette);
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
            HitAction::TabLyrics => {
                self.active_view = View::Lyrics;
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
            HitAction::ToggleShuffle => self.toggle_shuffle(),
            HitAction::SearchSelect(idx) => {
                self.search_state.selected_idx = idx;
                if let Some(track) = self.search_state.selected_track().cloned() {
                    let _ = self.play_track_file(&track.path);
                    self.search_state.close();
                    self.active_view = View::Player;
                    self.last_art_rendered = None;
                }
            }
            HitAction::SearchClose => {
                self.search_state.close();
                self.last_art_rendered = None;
            }
            HitAction::SeekLyric(ts) => {
                self.audio_engine.seek_to(ts);
                self.player.position = ts;
                let active_idx = self.lyrics_state.lyrics.as_ref().and_then(|l| l.find_active_index(ts));
                self.lyrics_state.resume_auto_scroll(active_idx);
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

        if self.search_state.is_open {
            match key.code {
                KeyCode::Esc => {
                    self.search_state.close();
                    self.last_art_rendered = None;
                }
                KeyCode::Enter => {
                    if let Some(track) = self.search_state.selected_track().cloned() {
                        let _ = self.play_track_file(&track.path);
                        self.search_state.close();
                        self.active_view = View::Player;
                        self.last_art_rendered = None;
                    }
                }
                KeyCode::Up => self.search_state.move_up(),
                KeyCode::Down => self.search_state.move_down(),
                KeyCode::Backspace => self.search_state.backspace(),
                KeyCode::Char(c) => {
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        if c == 'c' {
                            self.should_quit = true;
                        } else if c == 'u' {
                            self.search_state.query.clear();
                            self.search_state.refresh_results();
                        }
                    } else {
                        self.search_state.type_char(c);
                    }
                }
                _ => {}
            }
            return;
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }

        // Global search toggle with '/'
        if key.code == KeyCode::Char('/') {
            let mut pool = Vec::new();
            let mut seen = std::collections::HashSet::new();

            for t in &self.queue.tracks {
                if seen.insert(t.path.clone()) {
                    pool.push(t.clone());
                }
            }

            for t in &self.library_state.track_entries {
                if seen.insert(t.path.clone()) {
                    pool.push(t.clone());
                }
            }

            let _ = KittyRenderer::clear_all();
            self.last_art_rendered = None;
            self.search_state.open(pool);
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
            KeyCode::Char('4') => {
                self.active_view = View::Lyrics;
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
                    KeyCode::Char('s') => self.toggle_shuffle(),
                    KeyCode::Char('q') | KeyCode::Esc => self.active_view = View::Player,
                    _ => {}
                }
                return;
            }
            View::Lyrics => {
                let total = self.lyrics_state.lyrics.as_ref().map(|l| l.lines.len()).unwrap_or(0);
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => self.lyrics_state.move_up(),
                    KeyCode::Down | KeyCode::Char('j') => self.lyrics_state.move_down(total),
                    KeyCode::Enter => {
                        if let Some(ts) = self.lyrics_state.selected_timestamp() {
                            self.audio_engine.seek_to(ts);
                            self.player.position = ts;
                            let active_idx = self.lyrics_state.lyrics.as_ref().and_then(|l| l.find_active_index(ts));
                            self.lyrics_state.resume_auto_scroll(active_idx);
                        }
                    }
                    KeyCode::Char('s') => {
                        let active_idx = self.lyrics_state.lyrics.as_ref().and_then(|l| l.find_active_index(self.player.position));
                        self.lyrics_state.resume_auto_scroll(active_idx);
                    }
                    KeyCode::Char(' ') => self.toggle_play_pause(),
                    KeyCode::Char('n') => self.next_track(),
                    KeyCode::Char('p') => self.prev_track(),
                    KeyCode::Right => self.audio_engine.seek_relative(5),
                    KeyCode::Left => self.audio_engine.seek_relative(-5),
                    KeyCode::Char('+') | KeyCode::Char('=') => {
                        let _ = self.audio_engine.adjust_volume(0.05);
                    }
                    KeyCode::Char('-') => {
                        let _ = self.audio_engine.adjust_volume(-0.05);
                    }
                    KeyCode::Char('m') => {
                        let _ = self.audio_engine.toggle_mute();
                    }
                    KeyCode::Char('l') | KeyCode::Char('q') | KeyCode::Esc => self.active_view = View::Player,
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
            KeyCode::Char('s') => self.toggle_shuffle(),
            KeyCode::Char('l') => {
                self.active_view = View::Lyrics;
                let _ = KittyRenderer::clear_all();
                self.last_art_rendered = None;
            }
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
                if self.active_view == View::Lyrics {
                    self.lyrics_state.move_up();
                } else {
                    let _ = self.audio_engine.adjust_volume(0.05);
                }
            }
            MouseEventKind::ScrollDown => {
                if self.active_view == View::Lyrics {
                    let total = self.lyrics_state.lyrics.as_ref().map(|l| l.lines.len()).unwrap_or(0);
                    self.lyrics_state.move_down(total);
                } else {
                    let _ = self.audio_engine.adjust_volume(-0.05);
                }
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
                View::Lyrics => {
                    LyricsView::render(frame, area, &self.player, &mut self.lyrics_state, &self.theme, &mut self.hit_zones);
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

            if self.search_state.is_open {
                SearchOverlay::render(frame, area, &self.search_state, &self.theme, &mut self.hit_zones);
            } else if self.show_help {
                HelpOverlay::render(frame, area, &self.theme);
            }
        })?;

        // Render Kitty graphics album artwork only when on Player view
        if self.active_view == View::Player && !self.fullscreen_visualizer && !self.search_state.is_open && self.show_artwork && self.terminal_graphics == TerminalGraphics::Kitty {
            if let (Some(rect), Some(png)) = (art_box_rect, &self.artwork_png) {
                if self.last_art_rendered != Some(rect) {
                    let _ = KittyRenderer::clear_all();
                    let _ = KittyRenderer::render_png(png, rect.0, rect.1, rect.2, rect.3);
                    self.last_art_rendered = Some(rect);
                }
            }
        }

        Ok(())
    }

    pub fn save_config(&self) {
        let mut cfg = self.config.clone();
        cfg.player.volume = self.player.volume;
        cfg.player.repeat = match self.player.repeat {
            RepeatMode::Off => "off".to_string(),
            RepeatMode::All => "all".to_string(),
            RepeatMode::One => "one".to_string(),
        };
        cfg.player.shuffle = self.player.shuffle;
        cfg.ui.theme = self.theme.name.to_string();
        cfg.ui.artwork = self.show_artwork;
        cfg.ui.notifications = self.notifications_enabled;
        cfg.ui.visualizer = match self.visualizer_kind {
            VisualizerKind::Bars => "bars".to_string(),
            VisualizerKind::Mirrored => "mirrored".to_string(),
            VisualizerKind::Waveform => "waveform".to_string(),
            VisualizerKind::VuMeter => "vu".to_string(),
            VisualizerKind::Waterfall => "waterfall".to_string(),
            VisualizerKind::Particles => "particles".to_string(),
        };
        let _ = cfg.save();
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.save_config();
    }
}
