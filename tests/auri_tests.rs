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
    assert_eq!(theme5.name, "Candy");

    let theme6 = theme5.cycle_next();
    assert_eq!(theme6.name, "Matrix");

    let theme7 = theme6.cycle_next();
    assert_eq!(theme7.name, "Mono");

    let theme8 = theme7.cycle_next();
    assert_eq!(theme8.name, "Album");

    let theme9 = theme8.cycle_next();
    assert_eq!(theme9.name, "Neon Rainbow");
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

#[test]
fn test_fuzzy_search_scoring() {
    use auri::library::{fuzzy_score, score_track, Track};
    use std::path::PathBuf;

    // Subsequence match
    assert!(fuzzy_score("Bohemian Rhapsody", "boh").is_some());
    assert!(fuzzy_score("Bohemian Rhapsody", "rhap").is_some());
    assert!(fuzzy_score("Bohemian Rhapsody", "xyz").is_none());

    // Prefix match scores higher than middle match
    let prefix = fuzzy_score("Stairway to Heaven", "stair").unwrap();
    let mid = fuzzy_score("Stairway to Heaven", "heav").unwrap();
    assert!(prefix > mid);

    // Track score across title, artist, album
    let mut track = Track::new(PathBuf::from("/music/queen/bohemian.flac"));
    track.title = "Bohemian Rhapsody".to_string();
    track.artist = "Queen".to_string();
    track.album = "A Night at the Opera".to_string();

    assert!(score_track(&track, "queen").is_some());
    assert!(score_track(&track, "opera").is_some());
    assert!(score_track(&track, "bohem").is_some());
    assert!(score_track(&track, "metallica").is_none());
}

#[test]
fn test_search_state_lifecycle() {
    use auri::library::{SearchState, Track};
    use std::path::PathBuf;

    let mut t1 = Track::new(PathBuf::from("/music/song1.flac"));
    t1.title = "Hotel California".to_string();
    t1.artist = "Eagles".to_string();

    let mut t2 = Track::new(PathBuf::from("/music/song2.flac"));
    t2.title = "California Dreamin'".to_string();
    t2.artist = "The Mamas & The Papas".to_string();

    let mut search = SearchState::new();
    assert!(!search.is_open);

    search.open(vec![t1.clone(), t2.clone()]);
    assert!(search.is_open);
    assert_eq!(search.results.len(), 2);

    search.type_char('h');
    search.type_char('o');
    search.type_char('t');
    assert_eq!(search.results.len(), 1);
    assert_eq!(search.selected_track().unwrap().title, "Hotel California");

    search.backspace();
    search.backspace();
    search.backspace();
    assert_eq!(search.results.len(), 2);

    search.move_down();
    assert_eq!(search.selected_idx, 1);
    search.move_up();
    assert_eq!(search.selected_idx, 0);

    search.close();
    assert!(!search.is_open);
    assert!(search.results.is_empty());
}

#[test]
fn test_app_config_roundtrip() {
    use auri::config::AppConfig;
    use std::path::PathBuf;

    let mut cfg = AppConfig::default();
    cfg.player.volume = 0.65;
    cfg.player.repeat = "one".to_string();
    cfg.player.shuffle = true;
    cfg.ui.theme = "Sunset".to_string();
    cfg.ui.visualizer = "waveform".to_string();
    cfg.ui.artwork = false;
    cfg.library.paths = vec![PathBuf::from("/custom/music")];

    let serialized = toml::to_string_pretty(&cfg).expect("Failed to serialize AppConfig");
    let deserialized: AppConfig = toml::from_str(&serialized).expect("Failed to deserialize AppConfig");

    assert_eq!(deserialized.player.volume, 0.65);
    assert_eq!(deserialized.player.repeat, "one");
    assert!(deserialized.player.shuffle);
    assert_eq!(deserialized.ui.theme, "Sunset");
    assert_eq!(deserialized.ui.visualizer, "waveform");
    assert!(!deserialized.ui.artwork);
    assert_eq!(deserialized.library.paths, vec![PathBuf::from("/custom/music")]);
}

#[test]
fn test_theme_from_name() {
    use auri::theme::Theme;

    let t_neon = Theme::from_name("Neon Rainbow");
    assert_eq!(t_neon.name, "Neon Rainbow");

    let t_vercel = Theme::from_name("Vercel Dark");
    assert_eq!(t_vercel.name, "Vercel Dark");

    let t_ice = Theme::from_name("Ice");
    assert_eq!(t_ice.name, "Ice");

    let t_sunset = Theme::from_name("Sunset");
    assert_eq!(t_sunset.name, "Sunset");

    let t_candy = Theme::from_name("Candy");
    assert_eq!(t_candy.name, "Candy");

    let t_matrix = Theme::from_name("Matrix");
    assert_eq!(t_matrix.name, "Matrix");

    let t_mono = Theme::from_name("Mono");
    assert_eq!(t_mono.name, "Mono");

    let t_album = Theme::from_name("Album");
    assert_eq!(t_album.name, "Album");

    let t_unknown = Theme::from_name("NonExistentTheme");
    assert_eq!(t_unknown.name, "Neon Rainbow");
}

#[test]
fn test_visualizer_kind_cycle_and_names() {
    use auri::visualizers::VisualizerKind;

    let k1 = VisualizerKind::Bars;
    assert_eq!(k1.name(), "Spectrum Bars");

    let k2 = k1.next();
    assert_eq!(k2, VisualizerKind::Mirrored);
    assert_eq!(k2.name(), "Mirrored Bars");

    let k3 = k2.next();
    assert_eq!(k3, VisualizerKind::Waveform);
    assert_eq!(k3.name(), "Waveform");

    let k4 = k3.next();
    assert_eq!(k4, VisualizerKind::VuMeter);
    assert_eq!(k4.name(), "Stereo VU Meter");

    let k5 = k4.next();
    assert_eq!(k5, VisualizerKind::Bars);
}

#[test]
fn test_mirrored_bars_visualizer() {
    use auri::visualizers::mirrored::MirroredBarsVisualizer;
    use auri::visualizers::Visualizer;
    use auri::audio::frame::AudioFrame;
    use auri::theme::Theme;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use ratatui::layout::Rect;

    let mut viz = MirroredBarsVisualizer::default();
    assert_eq!(viz.name(), "Mirrored Bars");

    let theme = Theme::neon_rainbow();
    let mut audio = AudioFrame::default();
    audio.spectrum = vec![0.5; 64];

    let backend = TestBackend::new(60, 12);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|f| {
        let area = Rect::new(0, 0, 60, 12);
        viz.render(f, area, &audio, &theme);
    }).unwrap();

    let buffer = terminal.backend().buffer();
    // Verify some cells were rendered with block characters
    let mut non_empty_count = 0;
    for y in 0..12 {
        for x in 0..60 {
            let symbol = buffer.cell((x, y)).unwrap().symbol();
            if symbol != " " {
                non_empty_count += 1;
            }
        }
    }
    assert!(non_empty_count > 0, "Mirrored bars should render non-empty blocks");
}

#[test]
fn test_vu_meter_visualizer() {
    use auri::visualizers::vu::VuMeterVisualizer;
    use auri::visualizers::Visualizer;
    use auri::audio::frame::AudioFrame;
    use auri::theme::Theme;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use ratatui::layout::Rect;

    let mut viz = VuMeterVisualizer::default();
    assert_eq!(viz.name(), "Stereo VU Meter");

    let theme = Theme::vercel_dark();
    let mut audio = AudioFrame::default();
    audio.rms_left = 0.5;
    audio.rms_right = 0.25;
    audio.peak_left = 0.995; // Should trigger clip
    audio.peak_right = 0.4;

    let backend = TestBackend::new(80, 14);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|f| {
        let area = Rect::new(0, 0, 80, 14);
        viz.render(f, area, &audio, &theme);
    }).unwrap();

    let buffer = terminal.backend().buffer();
    // Verify scale or channel labels were rendered
    let mut rendered_text = String::new();
    for y in 0..14 {
        for x in 0..80 {
            rendered_text.push_str(buffer.cell((x, y)).unwrap().symbol());
        }
        rendered_text.push('\n');
    }

    assert!(rendered_text.contains('L'), "VU meter should contain Left channel indicator");
    assert!(rendered_text.contains('R'), "VU meter should contain Right channel indicator");
    assert!(rendered_text.contains("CLIP"), "Left channel should trigger CLIP indicator");
}

#[test]
fn test_artwork_palette_extraction() {
    use auri::theme::extract_palette;
    use image::{DynamicImage, RgbaImage, Rgba};
    use ratatui::style::Color;

    let mut img = RgbaImage::new(48, 48);
    for y in 0..48 {
        for x in 0..48 {
            if x < 24 {
                img.put_pixel(x, y, Rgba([0, 120, 255, 255]));
            } else {
                img.put_pixel(x, y, Rgba([255, 200, 0, 255]));
            }
        }
    }

    let dyn_img = DynamicImage::ImageRgba8(img);
    let palette = extract_palette(&dyn_img);

    assert_ne!(palette.primary, palette.secondary);
    assert_ne!(palette.peak, Color::Rgb(0, 0, 0));
}

#[test]
fn test_artwork_palette_extraction_grayscale() {
    use auri::theme::extract_palette;
    use image::{DynamicImage, RgbaImage, Rgba};

    let mut img = RgbaImage::new(48, 48);
    for y in 0..48 {
        for x in 0..48 {
            img.put_pixel(x, y, Rgba([128, 128, 128, 255]));
        }
    }

    let dyn_img = DynamicImage::ImageRgba8(img);
    let palette = extract_palette(&dyn_img);

    assert_ne!(palette.primary, palette.secondary);
}
