<p align="center">
  <img src="logo.png" alt="Auri logo" width="300">
</p>

# Auri

A terminal music player and audio visualizer for local audio files.

Auri combines real-time spectrum analysis, embedded album artwork via the Kitty graphics protocol, interactive playlists and library browsing, and a responsive keyboard- and touch-friendly interface built with Ratatui.

## Features

- **Local Audio Playback:** Decodes MP3, FLAC, WAV, OGG, and AAC/M4A with pure Rust decoders via Symphonia and outputs low-latency audio through CPAL.
- **Neon Rainbow Visualizers:** Multi-stop frequency spectrum equalizer bars with peak caps, smooth decay, and time-domain oscilloscope waveforms.
- **Kitty Album Artwork:** Renders embedded album art and local folder covers directly in Kitty, Ghostty, and WezTerm terminals with graceful fallback.
- **Interactive Library & Playlists:** Dual-column file and playlist browser (`2`) to open folders, play albums, enqueue tracks, and manage `.m3u` / `.m3u8` playlists.
- **Active Queue Management:** Inspect the queue (`3`), reorder, remove items (`d`), clear (`c`), or shuffle (`s`).
- **Synchronized Lyrics:** Reads companion `.lrc` files and embedded lyric tags with real-time center-scrolling and tap-to-seek (`4` or `l`).
- **Desktop Notifications:** Displays track title, artist, album, and duration on track changes via desktop notifications.
- **Linux MPRIS / D-Bus Integration:** Control playback with desktop media keys, `playerctl`, GNOME/KDE media widgets, and lock screens.
- **Audio Output Device Selector:** Press `o` / `O` or tap the device badge to switch audio output devices (headphones, DACs, HDMI) on the fly.
- **Full-Screen Visualizer Mode:** Press `f` for a focused visualizer experience with a minimal now-playing footer.
- **Touch & Mouse Support:** Click or tap progress bars to seek, drag volume, tap transport buttons, switch tabs, or cycle visualizers.
- **Sleek Minimal Aesthetics:** Charcoal dark palette (`#121214`) with crisp typography and vibrant accent palettes.

## Requirements

- Rust 1.75+ (tested on Rust 1.96)
- ALSA development libraries on Linux (`libasound2-dev` on Debian/Ubuntu, `alsa-lib-devel` on Fedora)
- A modern terminal emulator (Kitty, Ghostty, or WezTerm for album artwork; any UTF-8 terminal for playback and visualizers)

## Installation & Build

Clone the repository and build with Cargo:

```bash
git clone https://github.com/pinkpixel-dev/auri.git
cd auri
cargo build --release
```

The compiled binary will be placed at `./target/release/auri`.

## Quick Start

Play an individual audio file:

```bash
auri path/to/song.flac
```

Play a folder containing albums:

```bash
auri ~/Music/Synthwave/
```

Load an M3U or M3U8 playlist:

```bash
auri favorites.m3u
```

Launch without arguments to load your system music directory (`~/Music`):

```bash
auri
```

## Keybindings & Controls

### Playback

| Key | Touch / Mouse Action | Description |
| :--- | :--- | :--- |
| `Space` | Tap Play/Pause button | Toggle play and pause |
| `n` | Tap Next button | Next track in queue |
| `p` | Tap Previous button | Previous track in queue |
| `←` / `→` | Tap Timeline bar | Seek backward / forward 5 seconds |
| `↑` / `↓` | Tap Volume bar / Scroll | Adjust volume by 5% |
| `m` | - | Toggle mute |
| `r` | Tap Repeat label | Cycle repeat mode (All, One, Off) |
| `s` | Tap Shuffle label | Toggle shuffle mode |
| `o` / `O` | Tap Audio Device badge | Open audio output device selector |

### Visuals & Views

| Key | Touch / Mouse Action | Description |
| :--- | :--- | :--- |
| `f` | Tap Artwork or Visualizer | Toggle full-screen visualizer mode |
| `v` | Tap Visualizer box | Cycle visualizer (Spectrum Bars, Waveform) |
| `t` | - | Cycle theme (Neon Rainbow, Vercel Dark, Ice, Sunset, Mono) |
| `a` | - | Toggle album artwork display |
| `1` | Tap Player tab | Switch to Player view |
| `2` | Tap Library tab | Switch to Library and Playlist browser |
| `3` | Tap Queue tab | Switch to Playback Queue manager |
| `4` / `l` | Tap Lyrics tab | Switch to Synchronized Lyrics view |
| `?` | Tap Help tab | Show or hide the keybindings overlay |
| `q` / `Ctrl+C` | - | Quit Auri |

### Library View (`2`)

| Key | Description |
| :--- | :--- |
| `↑` / `↓` or `k` / `j` | Move selection up or down |
| `←` / `→` or `h` / `l` | Switch between Folders/Playlists and Tracks panels |
| `Enter` | Open folder, load playlist, or play selected track |
| `a` | Add selected track or entire folder to queue |
| `p` | Save active queue as a new playlist |
| `1` | Return to Player view |

### Queue View (`3`)

| Key | Description |
| :--- | :--- |
| `↑` / `↓` or `k` / `j` | Move selection up or down |
| `Enter` | Jump directly to selected track |
| `d` / `Delete` | Remove selected track from queue |
| `c` | Clear entire queue |
| `s` | Shuffle queue order |
| `1` | Return to Player view |

### Lyrics View (`4` or `l`)

| Key | Touch / Mouse Action | Description |
| :--- | :--- | :--- |
| `↑` / `↓` or `k` / `j` | Mouse wheel scroll | Browse lyric lines manually (pauses auto-scroll) |
| `Enter` | Tap any line | Seek playback directly to that timestamp |
| `s` | Tap re-sync badge | Snap back to active line and resume auto-scroll |
| `Space` | Tap Play/Pause button | Toggle playback |
| `n` / `p` | Tap Next/Prev button | Next or previous track |
| `1` / `l` / `Esc` | Tap Player tab | Return to Player view |

### Linux MPRIS & Media Keys

Auri implements the standard MPRIS D-Bus interface (`org.mpris.MediaPlayer2` and `org.mpris.MediaPlayer2.Player`). You can control playback using your desktop keyboard media keys, lock screen widgets, or `playerctl`:

```bash
playerctl play-pause
playerctl next
playerctl previous
playerctl position 30
playerctl metadata
```

## Architecture

Auri separates playback, analysis, and rendering into distinct layers:

1. **Audio Engine (`src/audio/`):** A background decode worker continuously buffers PCM samples from Symphonia into a ring buffer. The CPAL audio stream drains the buffer with volume scaling.
2. **Analysis Pipeline (`src/audio/analysis.rs`):** Applies a Hann window to recent PCM samples, executes forward FFT via `rustfft`, groups frequencies logarithmically into weighted bins, and applies fast-attack and smooth-decay filtering.
3. **Playlists & Library (`src/player/playlist.rs`, `src/ui/library.rs`):** Parses and writes standard M3U/M3U8 playlists, manages user playlists in `~/.config/auri/playlists/`, and provides folder navigation.
4. **Ratatui Interface (`src/ui/`):** Computes responsive layout areas, renders custom widgets directly to the terminal buffer, and collects click hit zones for mobile/mouse interactions.
5. **Kitty Protocol (`src/terminal/`):** Transmits base64-encoded PNG image chunks directly to the terminal window and manages cleanup on view changes.
6. **Linux MPRIS Service (`src/mpris.rs`):** Serves `org.mpris.MediaPlayer2` on session D-Bus, routing player actions across an unbounded crossbeam channel to the main loop with zero audio or render latency.
7. **Audio Output Device Manager (`src/audio/output.rs`, `src/ui/device.rs`):** Dynamically enumerates output endpoints and hot-swaps CPAL streams without interrupting the background decoder worker or clearing ring-buffer samples.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
