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

### 🏷️ Versioning
- Bump project version to 0.2.0.
