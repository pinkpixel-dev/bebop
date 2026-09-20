# Changelog

All notable changes to Auri are documented in this file.

## 0.2.0 - September 20, 2026

### 🎵 Lyrics
- Add synchronized lyric scrolling engine supporting standard `.lrc` files.
- Support millisecond precision timestamps, time offsets, and multi-timestamp lyrics lines.
- Automatically discover companion `.lrc` files in the track's folder and fallback to embedded tags.
- Add dedicated `LyricsView` (Tab 4, accessible via `4` or `l`) with center-scrolling active line highlighting.
- Add tap-to-seek and click-to-seek on any lyric line across mobile, touch, and desktop.
- Add manual scroll pausing with auto-scroll re-sync on `s` or `Enter`.
- Add mini player transport and progress deck within the lyrics screen.

### 🔔 Notifications
- Add desktop notifications on track change displaying title, artist, album, and duration.
- Dispatch notifications in detached background threads to ensure zero latency impact on playback and UI.
- Add configuration toggle `notifications = true/false` under `[ui]` in `config.toml`.
- Gracefully handle environments without a running D-Bus notification server.

### 🎛️ MPRIS & D-Bus
- Add Linux MPRIS integration exposing `org.mpris.MediaPlayer2` and `org.mpris.MediaPlayer2.Player` on the session D-Bus.
- Support external control via desktop keyboard media keys, `playerctl`, lock screen widgets, and GNOME/KDE media overlays.
- Decouple D-Bus actions into an unbounded crossbeam channel processed in the main event loop with zero UI or audio latency.
- Provide full playback control: play, pause, play-pause toggle, stop, next track, previous track, seek, position, volume, and quit.
- Expose synchronized track metadata dictionary including title, artist, album, duration, and file URI.
- Add configuration toggle `mpris = true/false` under `[ui]` in `config.toml`.
- Gracefully detect headless servers, SSH sessions, and environments without D-Bus.

### 🎧 Audio Output Device Selector
- Add interactive modal overlay for audio output device selection accessible via `o` / `O` or tapping the status bar audio badge.
- Enumerate host audio output endpoints through CPAL, marking the system default and active device.
- Hot-swap CPAL output streams at runtime while keeping the background decoder thread, PCM ring buffer, volume, and playback state uninterrupted.
- Persist preferred device name in `config.toml` under `[player.device]`.
- Provide touch and mouse hit zone support for direct device selection and tap-to-open.
- Automatically fall back to the system default device if a preferred device is unplugged or unavailable.

### 🏷️ Versioning
- Bump project version to 0.2.0.
