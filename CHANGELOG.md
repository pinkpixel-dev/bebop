# Changelog

All notable changes to Bebop are documented in this file.

## 1.0.0 - September 20, 2026

### 🎵 First 1.0 release
- Release Bebop 1.0.0 as a terminal music player for local MP3, FLAC, WAV, OGG, AAC, and M4A files.
- Include library browsing, queue management, M3U/M3U8 playlists, synchronized lyrics, album artwork, multiple visualizers, themes, and the interactive pixel cat.
- Support keyboard, mouse, and touch controls across the player, library, queue, lyrics, output device selector, and help views.

### 🎧 Desktop playback
- Include selectable audio output devices, desktop notifications, and Linux MPRIS controls for media keys, `playerctl`, desktop widgets, and lock screens.
- Resample decoded audio to the active output stream rate so files retain the correct pitch and playback speed.

### 🏷️ Bebop rename
- Rename the application, executable, repository path, MPRIS identity, and configuration directory from Auri to Bebop.
- Store settings in `~/.config/bebop/config.toml` and saved playlists in `~/.config/bebop/playlists/`. Existing Auri data is not moved automatically.

### 🏷️ Versioning
- Bump the project version from 0.3.0 to 1.0.0.

## 0.3.0 - September 20, 2026

### 🐾 Pet
- Pet the cat by clicking or tapping the pet pane, or by pressing `b`. It wakes up, bounces on the dancing frames with brighter blush, and floats hearts where the music notes usually sit. The reaction lasts about a second and restarts if you pet it again.
- Clicking the pet pane now pets the cat instead of hiding it. Hiding still works from the `🐾 [x]` status bar badge and the `x` key.

## 0.2.2 - September 20, 2026

### 🐛 Fixes
- Fix playback pitch and speed on files whose sample rate does not match the output stream. The stream was hardcoded to 44.1 kHz while decoded audio was queued at the file's own rate, so a 48 kHz track played at 0.919x speed, about a semitone and a half flat.
- Add `LinearResampler` and convert decoded audio to the output stream's rate in the decode worker.
- Open the output stream at 48 kHz when the device supports it, so the common case needs no conversion at all. ALSA plug devices such as `default` report an arbitrary rate as their default while accepting almost any rate, so that reported value is not reliable on its own.
- Rebuild the resampler when switching to an output device with a different rate, so a device change mid-track no longer shifts pitch.
- Feed the analyzer the output stream's rate instead of an assumed 44.1 kHz, which puts visualizer frequency bins back on the right frequencies.

## 0.2.1 - September 20, 2026

### 🎚️ Visualizers
- Map spectrum bins to dBFS instead of raw linear amplitude, so bars use the full height of the panel at normal listening levels.
- Replace the linear high-frequency multiplier with a dB tilt, keeping treble visible without letting it slam the ceiling.
- Add a contrast curve after the dB mapping so peaks still stand out from ordinary midrange content.

### 🐛 Fixes
- Fix the particle field pinning every particle to the top row. Updrafts now come from bass transients above a rolling baseline instead of any bass above a fixed threshold, so a steadily loud mix no longer reads as one continuous impulse.
- Give particles altitude-dependent drag and a steady lift tied to overall energy, so the field hovers, bounces on beats, rises through loud passages, and settles on the floor in silence.
- Rescale particle symbol thresholds and the treble sway for the new spectrum range.
- Fix the waterfall spectrogram painting a solid sheet of color. Cell color now mixes from the background through the theme hue to the peak color as energy rises, instead of being decided by horizontal position alone.
- Add a display floor and contrast curve to the waterfall so quiet content falls back to the background and bright ridges mark real peaks.

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

### 🐾 Dancing Pixel Pet
- Add audio-reactive dancing cat drawn in ANSI half-blocks, with four bounce frames that speed up with playback tempo and bass energy.
- Draw the cat with triangular ears, eyes with a highlight, a nose and muzzle, whiskers, blush, a collar, and a tail, so it reads as a cat instead of a blob.
- Add three sprite sizes (16x16, 12x12, and 10x8) and pick the largest one that fits the pane, so the cat grows with the terminal instead of sitting in empty space.
- Add a two-frame breathing sleep animation with the fur, collar, and headphones dimmed when playback is paused or stopped.
- Integrate pet as a dedicated symmetrical box on the right side of the bottom player deck (`[Artwork] [Controls] [Pet]`).
- Add toggle shortcut `x` and touch hit zone support on both the pet box and status bar badge (`🐾 [x]`).
- Collapse side boxes automatically on narrow/mobile viewports (< 55 columns) to prioritize transport controls.
- Persist pet toggle visibility preference in `config.toml` under `[ui.pet]`.

### 🖼️ Bottom Deck Layout
- Grow the bottom deck from a fixed 8 or 9 rows to between 8 and 13 rows, depending on terminal height, so album art and the pet get a bigger box.
- Narrow both side boxes when they cannot fit at full size, instead of dropping the pet, so an 80 column terminal keeps artwork and pet side by side.
- Keep at least 6 rows for the visualizer and 30 columns for the transport controls at every deck size.

### 🏷️ Versioning
- Bump project version to 0.2.0.
