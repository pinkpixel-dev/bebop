use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use crossbeam_channel::{Receiver, Sender};
use zbus::blocking::connection::Builder;
use zbus::interface;
use zbus::zvariant::{ObjectPath, Value};

use crate::audio::PlaybackState;
use crate::library::Track;
use crate::player::{PlayerState, RepeatMode};

#[derive(Clone, Debug, PartialEq)]
pub enum MprisAction {
    Play,
    Pause,
    PlayPause,
    Stop,
    Next,
    Previous,
    Seek(i64), // relative microseconds
    SetPosition(Duration),
    SetVolume(f64),
    Quit,
}

#[derive(Clone, Debug, Default)]
pub struct MprisState {
    pub playback_status: String,
    pub loop_status: String,
    pub shuffle: bool,
    pub volume: f64,
    pub position: Duration,
    pub track_title: String,
    pub track_artist: String,
    pub track_album: String,
    pub track_duration: Duration,
    pub track_path: Option<String>,
}

impl MprisState {
    pub fn metadata_dict(&self) -> HashMap<String, Value<'static>> {
        let mut map = HashMap::new();
        let track_id = ObjectPath::from_static_str("/org/mpris/MediaPlayer2/CurrentTrack")
            .unwrap_or_else(|_| ObjectPath::from_static_str("/").unwrap());
        map.insert("mpris:trackid".to_string(), Value::from(track_id));
        map.insert(
            "mpris:length".to_string(),
            Value::from(self.track_duration.as_micros() as i64),
        );
        map.insert(
            "xesam:title".to_string(),
            Value::from(self.track_title.clone()),
        );
        map.insert(
            "xesam:artist".to_string(),
            Value::from(vec![self.track_artist.clone()]),
        );
        map.insert(
            "xesam:album".to_string(),
            Value::from(self.track_album.clone()),
        );
        if let Some(path) = &self.track_path {
            map.insert(
                "xesam:url".to_string(),
                Value::from(format!("file://{}", path)),
            );
        }
        map
    }
}

pub struct MprisRoot {
    action_tx: Sender<MprisAction>,
}

#[interface(name = "org.mpris.MediaPlayer2")]
impl MprisRoot {
    fn raise(&self) {}

    fn quit(&self) {
        let _ = self.action_tx.send(MprisAction::Quit);
    }

    #[zbus(property)]
    fn can_quit(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_raise(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn has_track_list(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn identity(&self) -> &str {
        "Bebop"
    }

    #[zbus(property)]
    fn supported_uri_schemes(&self) -> Vec<&str> {
        vec!["file"]
    }

    #[zbus(property)]
    fn supported_mime_types(&self) -> Vec<&str> {
        vec![
            "audio/flac",
            "audio/mpeg",
            "audio/ogg",
            "audio/wav",
            "audio/aac",
            "audio/mp4",
        ]
    }
}

pub struct MprisPlayer {
    action_tx: Sender<MprisAction>,
    state: Arc<Mutex<MprisState>>,
}

#[interface(name = "org.mpris.MediaPlayer2.Player")]
impl MprisPlayer {
    fn next(&self) {
        let _ = self.action_tx.send(MprisAction::Next);
    }

    fn previous(&self) {
        let _ = self.action_tx.send(MprisAction::Previous);
    }

    fn pause(&self) {
        let _ = self.action_tx.send(MprisAction::Pause);
    }

    fn play_pause(&self) {
        let _ = self.action_tx.send(MprisAction::PlayPause);
    }

    fn stop(&self) {
        let _ = self.action_tx.send(MprisAction::Stop);
    }

    fn play(&self) {
        let _ = self.action_tx.send(MprisAction::Play);
    }

    fn seek(&self, offset_usec: i64) {
        let _ = self.action_tx.send(MprisAction::Seek(offset_usec));
    }

    fn set_position(&self, _track_id: ObjectPath<'_>, position_usec: i64) {
        if position_usec >= 0 {
            let _ = self.action_tx.send(MprisAction::SetPosition(Duration::from_micros(position_usec as u64)));
        }
    }

    fn open_uri(&self, _uri: &str) {}

    #[zbus(property)]
    fn playback_status(&self) -> String {
        self.state.lock().unwrap().playback_status.clone()
    }

    #[zbus(property)]
    fn loop_status(&self) -> String {
        self.state.lock().unwrap().loop_status.clone()
    }

    #[zbus(property)]
    fn set_loop_status(&self, _loop_status: String) {}

    #[zbus(property)]
    fn rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    fn set_rate(&self, _rate: f64) {}

    #[zbus(property)]
    fn shuffle(&self) -> bool {
        self.state.lock().unwrap().shuffle
    }

    #[zbus(property)]
    fn set_shuffle(&self, _shuffle: bool) {}

    #[zbus(property)]
    fn metadata(&self) -> HashMap<String, Value<'static>> {
        self.state.lock().unwrap().metadata_dict()
    }

    #[zbus(property)]
    fn volume(&self) -> f64 {
        self.state.lock().unwrap().volume
    }

    #[zbus(property)]
    fn set_volume(&self, volume: f64) {
        let _ = self.action_tx.send(MprisAction::SetVolume(volume));
    }

    #[zbus(property)]
    fn position(&self) -> i64 {
        self.state.lock().unwrap().position.as_micros() as i64
    }

    #[zbus(property)]
    fn minimum_rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    fn maximum_rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    fn can_go_next(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_go_previous(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_play(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_pause(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_seek(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_control(&self) -> bool {
        true
    }
}

pub struct MprisService {
    pub action_rx: Receiver<MprisAction>,
    shared_state: Arc<Mutex<MprisState>>,
    exit_flag: Arc<AtomicBool>,
}

impl Drop for MprisService {
    fn drop(&mut self) {
        self.exit_flag.store(true, Ordering::SeqCst);
    }
}

impl MprisService {
    /// Initialize the MPRIS service on the session D-Bus.
    /// Returns `None` if connecting to D-Bus fails (e.g. headless/TTY environments).
    pub fn new() -> Option<Self> {
        let (action_tx, action_rx) = crossbeam_channel::unbounded();
        let shared_state = Arc::new(Mutex::new(MprisState::default()));
        let exit_flag = Arc::new(AtomicBool::new(false));

        let state_clone = Arc::clone(&shared_state);
        let tx_clone = action_tx.clone();
        let exit_clone = Arc::clone(&exit_flag);

        let handle = thread::Builder::new()
            .name("bebop-mpris".to_string())
            .spawn(move || {
                let root = MprisRoot {
                    action_tx: tx_clone.clone(),
                };
                let player = MprisPlayer {
                    action_tx: tx_clone,
                    state: state_clone,
                };

                let conn_res = Builder::session()
                    .and_then(|b| b.name("org.mpris.MediaPlayer2.bebop"))
                    .and_then(|b| b.serve_at("/org/mpris/MediaPlayer2", root))
                    .and_then(|b| b.serve_at("/org/mpris/MediaPlayer2", player))
                    .and_then(|b| b.build());

                if let Ok(_conn) = conn_res {
                    while !exit_clone.load(Ordering::SeqCst) {
                        thread::sleep(Duration::from_millis(200));
                    }
                }
            });

        if handle.is_err() {
            return None;
        }

        Some(Self {
            action_rx,
            shared_state,
            exit_flag,
        })
    }

    /// Update the shared playback and track state exposed via D-Bus properties.
    pub fn update_state(&self, player: &PlayerState, track: Option<&Track>) {
        if let Ok(mut state) = self.shared_state.lock() {
            state.playback_status = match player.playback_state {
                PlaybackState::Playing => "Playing".to_string(),
                PlaybackState::Paused => "Paused".to_string(),
                PlaybackState::Stopped => "Stopped".to_string(),
            };
            state.loop_status = match player.repeat {
                RepeatMode::Off => "None".to_string(),
                RepeatMode::One => "Track".to_string(),
                RepeatMode::All => "Playlist".to_string(),
            };
            state.shuffle = player.shuffle;
            state.volume = if player.muted { 0.0 } else { player.volume as f64 };
            state.position = player.position;

            if let Some(t) = track {
                state.track_title = t.title.clone();
                state.track_artist = t.artist.clone();
                state.track_album = t.album.clone();
                state.track_duration = player.duration;
                state.track_path = Some(t.path.to_string_lossy().to_string());
            } else {
                state.track_title.clear();
                state.track_artist.clear();
                state.track_album.clear();
                state.track_duration = Duration::ZERO;
                state.track_path = None;
            }
        }
    }
}
