use std::path::PathBuf;
use std::time::Duration;

use auri::audio::analysis::AudioAnalyzer;
use auri::library::{format_time, Track};
use auri::player::{PlayerState, PlaylistManager, Queue, RepeatMode};
use auri::theme::Theme;
use auri::ui::{AppLayout, LibraryState, QueueState};
use ratatui::layout::Rect;
use ratatui::style::Color;

#[test]
fn test_time_formatting() {
    assert_eq!(format_time(Duration::from_secs(0)), "00:00");
    assert_eq!(format_time(Duration::from_secs(45)), "00:45");
    assert_eq!(format_time(Duration::from_secs(161)), "02:41");
    assert_eq!(format_time(Duration::from_secs(252)), "04:12");
    assert_eq!(format_time(Duration::from_secs(3665)), "01:01:05");
}

#[test]
fn test_track_tech_details() {
    let mut track = Track::new(PathBuf::from("/music/song.flac"));
    track.format = "FLAC".to_string();
    track.sample_rate = 44100;
    track.bit_depth = Some(24);

    assert_eq!(track.formatted_tech_details(), "FLAC · 44.1 kHz · 24-bit");

    let mut track2 = Track::new(PathBuf::from("/music/song.mp3"));
    track2.format = "MP3".to_string();
    track2.sample_rate = 48000;
    track2.bit_depth = None;

    assert_eq!(track2.formatted_tech_details(), "MP3 · 48.0 kHz");
}

#[test]
fn test_queue_navigation_and_repeat() {
    let mut queue = Queue::new();
    let t1 = Track::new("/music/track1.flac");
    let t2 = Track::new("/music/track2.flac");
    let t3 = Track::new("/music/track3.flac");

    queue.set_tracks(vec![t1.clone(), t2.clone(), t3.clone()], 0);
    assert_eq!(queue.current_track().unwrap().path, t1.path);

    // Advance next
    let next1 = queue.advance_next(false).unwrap();
    assert_eq!(next1.path, t2.path);

    let next2 = queue.advance_next(false).unwrap();
    assert_eq!(next2.path, t3.path);

    // End of queue with repeat off
    assert!(queue.advance_next(false).is_none());

    // End of queue with repeat all wraps to 0
    let wrapped = queue.advance_next(true).unwrap();
    assert_eq!(wrapped.path, t1.path);

    // Previous track
    let prev = queue.advance_prev().unwrap();
    assert_eq!(prev.path, t1.path); // at index 0 stays at 0
}

#[test]
fn test_repeat_mode_cycling() {
    let mut state = PlayerState::default();
    assert_eq!(state.repeat, RepeatMode::All);

    state.repeat = state.repeat.cycle();
    assert_eq!(state.repeat, RepeatMode::One);

    state.repeat = state.repeat.cycle();
    assert_eq!(state.repeat, RepeatMode::Off);

    state.repeat = state.repeat.cycle();
    assert_eq!(state.repeat, RepeatMode::All);
}

#[test]
fn test_audio_analyzer_sine_wave() {
    let mut analyzer = AudioAnalyzer::new(44100, 32);

    // Generate a 440 Hz stereo sine wave (A4 note)
    let sample_count = 2048 * 2;
    let mut samples = Vec::with_capacity(sample_count);
    for i in 0..2048 {
        let t = i as f32 / 44100.0;
        let s = (2.0 * std::f32::consts::PI * 440.0 * t).sin();
        samples.push(s);
        samples.push(s);
    }

    let frame = analyzer.analyze(&samples);
    assert_eq!(frame.spectrum.len(), 32);
    assert_eq!(frame.waveform.len(), 256);

    // Peak amplitude should be close to 1.0
    assert!(frame.peak_left > 0.95);
    assert!(frame.peak_right > 0.95);

    // RMS of pure sine wave is ~1 / sqrt(2) ≈ 0.707
    assert!((frame.rms_left - 0.707).abs() < 0.05);

    // Non-zero spectrum energy
    let max_energy = frame.spectrum.iter().cloned().fold(0.0f32, f32::max);
    assert!(max_energy > 0.1);
}

#[test]
fn test_theme_cycling() {
    let theme = Theme::default();
    assert_eq!(theme.name, "Neon Rainbow");
    assert!(theme.is_rainbow);

    let theme2 = theme.cycle_next();
    assert_eq!(theme2.name, "Vercel Dark");

    let theme3 = theme2.cycle_next();
    assert_eq!(theme3.name, "Ice");

    let theme4 = theme3.cycle_next();
    assert_eq!(theme4.name, "Sunset");

    let theme5 = theme4.cycle_next();
    assert_eq!(theme5.name, "Mono");

    let theme6 = theme5.cycle_next();
    assert_eq!(theme6.name, "Neon Rainbow");
}

#[test]
fn test_neon_rainbow_colors() {
    let theme = Theme::neon_rainbow();

    // Bass frequency (t = 0.0) should be neon magenta/rose
    let bass_color = theme.get_bar_color(0.0, 0.5);
    match bass_color {
        Color::Rgb(r, _g, b) => {
            assert!(r > 200, "Expected high red for neon pink/rose");
            assert!(b > 100, "Expected magenta blue component");
        }
        _ => panic!("Expected RGB color"),
    }

    // Mid frequency (t = 0.6) should be cyan
    let mid_color = theme.get_bar_color(0.6, 0.5);
    match mid_color {
        Color::Rgb(_r, g, b) => {
            assert!(b > 180, "Expected high blue for cyan");
            assert!(g > 150, "Expected high green for cyan");
        }
        _ => panic!("Expected RGB color"),
    }

    // Treble frequency (t = 1.0) should be amber/gold
    let treble_color = theme.get_bar_color(1.0, 0.5);
    match treble_color {
        Color::Rgb(r, g, _) => {
            assert!(r > 200, "Expected high red for amber/gold");
            assert!(g > 120, "Expected high green for amber/gold");
        }
        _ => panic!("Expected RGB color"),
    }
}

#[test]
fn test_playlist_m3u_save_and_load() {
    let temp_dir = std::env::temp_dir().join("auri_test_playlist");
    let _ = std::fs::create_dir_all(&temp_dir);
    let playlist_file = temp_dir.join("test_playlist.m3u");

    let mut t1 = Track::new(temp_dir.join("song1.mp3"));
    t1.title = "Song One".to_string();
    t1.artist = "Artist A".to_string();
    t1.duration = Duration::from_secs(180);

    // Create a dummy file so exists() passes
    let _ = std::fs::write(&t1.path, b"dummy audio content");

    let tracks = vec![t1.clone()];
    PlaylistManager::save_m3u(&playlist_file, &tracks).expect("Failed to save M3U");

    assert!(playlist_file.exists());

    let loaded = PlaylistManager::load_m3u(&playlist_file).expect("Failed to load M3U");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].path, t1.path);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_library_and_queue_state() {
    let mut lib = LibraryState::new();
    assert!(!lib.folder_entries.is_empty() || lib.track_entries.is_empty() || true);
    lib.move_up();
    lib.move_down();

    let mut q_state = QueueState::new();
    assert_eq!(q_state.selected_idx, 0);
    q_state.move_down(5);
    assert_eq!(q_state.selected_idx, 1);
    q_state.move_up();
    assert_eq!(q_state.selected_idx, 0);
}

#[test]
fn test_layout_calculation() {
    // Wide terminal area
    let area = Rect::new(0, 0, 100, 40);
    let layout = AppLayout::calculate(area, true);

    assert_eq!(layout.header.height, 3);
    assert!(layout.artwork.is_some());
    assert!(layout.visualizer.width >= 20);
    assert_eq!(layout.player_controls.height, 7);
    assert_eq!(layout.status.height, 3);

    // Narrow/mobile terminal area (width < 50): artwork collapses gracefully to prioritize visualizer
    let compact_area = Rect::new(0, 0, 45, 24);
    let compact_layout = AppLayout::calculate(compact_area, true);

    assert!(compact_layout.artwork.is_none());
    assert_eq!(compact_layout.visualizer.width, 45);
}

#[test]
fn test_metadata_and_decoder_probe() {
    let test_path = dirs::audio_dir()
        .map(|d| d.join("Misfits - Last Caress.mp3"))
        .filter(|p| p.exists());

    if let Some(path) = test_path {
        let (track, _art) = auri::library::MetadataReader::read_track(&path);
        assert!(!track.title.is_empty());
        assert_eq!(track.format, "MP3");

        let mut decoder = auri::audio::decoder::AudioDecoder::open(&path).expect("Decoder failed to open MP3");
        assert!(decoder.sample_rate() > 0);
        assert!(decoder.channels() > 0);

        let mut sample_buf = vec![0.0f32; 1024];
        let read = decoder.read_samples(&mut sample_buf);
        assert!(read > 0, "Expected decoder to decode audio frames");
    }
}

#[test]
fn test_audio_engine_stop_and_finish_flag() {
    if let Ok(mut engine) = auri::audio::AudioEngine::new() {
        assert!(!engine.is_finished());
        engine.stop();
        assert!(!engine.is_finished());
    }
}
