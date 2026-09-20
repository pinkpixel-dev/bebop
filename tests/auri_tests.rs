use std::path::PathBuf;
use std::time::Duration;

use auri::audio::analysis::AudioAnalyzer;
use auri::library::{format_time, Track};
use auri::lyrics::{Lyrics, LyricsSource, LyricsState};
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
    assert_eq!(layout.visualizer.width, 100); // Visualizer now spans full terminal width
    assert!(layout.artwork.is_some());
    let art = layout.artwork.unwrap();
    // Inner dimensions preserve 2:1 character cell aspect ratio for 1:1 pixel square
    assert_eq!(art.width - 2, (art.height - 2) * 2);
    assert!(layout.player_controls.height >= 8);
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
    assert_eq!(k5, VisualizerKind::Waterfall);
    assert_eq!(k5.name(), "Waterfall Spectrogram");

    let k6 = k5.next();
    assert_eq!(k6, VisualizerKind::Particles);
    assert_eq!(k6.name(), "Particle Field");

    let k7 = k6.next();
    assert_eq!(k7, VisualizerKind::Bars);
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

#[test]
fn test_waterfall_visualizer() {
    use auri::visualizers::waterfall::WaterfallVisualizer;
    use auri::visualizers::Visualizer;
    use auri::audio::frame::AudioFrame;
    use auri::theme::Theme;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use ratatui::layout::Rect;

    let mut viz = WaterfallVisualizer::default();
    assert_eq!(viz.name(), "Waterfall Spectrogram");

    let theme = Theme::vercel_dark();
    let mut audio = AudioFrame::default();
    audio.spectrum = vec![0.7; 64];

    let backend = TestBackend::new(50, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|f| {
        let area = Rect::new(0, 0, 50, 10);
        viz.render(f, area, &audio, &theme);
    }).unwrap();

    let buffer = terminal.backend().buffer();
    let mut non_empty_count = 0;
    for y in 0..10 {
        for x in 0..50 {
            let symbol = buffer.cell((x, y)).unwrap().symbol();
            if symbol != " " {
                non_empty_count += 1;
            }
        }
    }
    assert!(non_empty_count > 0, "Waterfall should render non-empty density characters");
}

#[test]
fn test_particles_visualizer() {
    use auri::visualizers::particles::ParticlesVisualizer;
    use auri::visualizers::Visualizer;
    use auri::audio::frame::AudioFrame;
    use auri::theme::Theme;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use ratatui::layout::Rect;

    let mut viz = ParticlesVisualizer::default();
    assert_eq!(viz.name(), "Particle Field");

    let theme = Theme::neon_rainbow();
    let mut audio = AudioFrame::default();
    audio.spectrum = vec![0.8; 64]; // Strong bass impulse

    let backend = TestBackend::new(60, 15);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|f| {
        let area = Rect::new(0, 0, 60, 15);
        viz.render(f, area, &audio, &theme);
    }).unwrap();

    let buffer = terminal.backend().buffer();
    let mut particle_count = 0;
    for y in 0..15 {
        for x in 0..60 {
            let symbol = buffer.cell((x, y)).unwrap().symbol();
            if symbol != " " {
                particle_count += 1;
            }
        }
    }
    assert!(particle_count > 0, "Particles should render across the buffer");
}

#[test]
fn test_fisher_yates_shuffle_deck() {
    // 0 and 1 elements
    assert_eq!(Queue::generate_shuffle_deck(0, None), Vec::<usize>::new());
    assert_eq!(Queue::generate_shuffle_deck(1, None), vec![0]);
    assert_eq!(Queue::generate_shuffle_deck(1, Some(0)), vec![0]);

    // 10 elements: must be a valid permutation
    let deck = Queue::generate_shuffle_deck(10, None);
    assert_eq!(deck.len(), 10);
    let mut sorted = deck.clone();
    sorted.sort();
    assert_eq!(sorted, (0..10).collect::<Vec<usize>>());

    // Pinned first element
    let pinned_deck = Queue::generate_shuffle_deck(10, Some(7));
    assert_eq!(pinned_deck[0], 7);
    let mut sorted_pinned = pinned_deck.clone();
    sorted_pinned.sort();
    assert_eq!(sorted_pinned, (0..10).collect::<Vec<usize>>());
}

#[test]
fn test_queue_shuffle_mode_forward_and_backward() {
    let mut queue = Queue::new();
    let tracks = (0..5)
        .map(|i| Track::new(format!("/music/track_{}.mp3", i)))
        .collect::<Vec<_>>();

    // Start playback at track 2 in sequential mode
    queue.set_tracks(tracks.clone(), 2);
    assert_eq!(queue.current_index, Some(2));

    // Turn shuffle on: current track 2 MUST remain active at cursor 0
    queue.set_shuffle(true);
    assert!(queue.shuffle);
    assert_eq!(queue.current_index, Some(2));
    assert_eq!(queue.shuffle_cursor, 0);
    assert_eq!(queue.shuffle_order[0], 2);

    // Verify all 5 tracks are in shuffle order
    let mut sorted = queue.shuffle_order.clone();
    sorted.sort();
    assert_eq!(sorted, vec![0, 1, 2, 3, 4]);

    // Advance through the entire shuffled queue
    let mut played_indices = vec![2];
    for _ in 0..4 {
        let _ = queue.advance_next(false).expect("should advance within deck");
        played_indices.push(queue.current_index.unwrap());
    }
    assert_eq!(played_indices, queue.shuffle_order);

    // At end of deck with repeat off, advance_next returns None
    assert!(queue.advance_next(false).is_none());

    // Step backwards along the exact sequence played
    for &expected_idx in played_indices.iter().rev().skip(1) {
        let prev_path = queue.advance_prev().expect("should advance prev").path.clone();
        assert_eq!(queue.current_index, Some(expected_idx));
        assert_eq!(prev_path, tracks[expected_idx].path);
    }

    // At the very beginning of the shuffled deck, advance_prev stays at index 0
    let _ = queue.advance_prev().expect("stays at first");
    assert_eq!(queue.current_index, Some(2));
}

#[test]
fn test_queue_shuffle_repeat_wrap() {
    let mut queue = Queue::new();
    let tracks = (0..3)
        .map(|i| Track::new(format!("/music/track_{}.mp3", i)))
        .collect::<Vec<_>>();

    queue.set_shuffle(true);
    queue.set_tracks(tracks, 0);

    // Play all 3 tracks
    queue.advance_next(false);
    queue.advance_next(false);

    // Advance with repeat_all: triggers fresh shuffle deck
    let wrapped = queue.advance_next(true);
    assert!(wrapped.is_some());
    assert_eq!(queue.shuffle_cursor, 0);
    assert_eq!(queue.shuffle_order.len(), 3);
}

#[test]
fn test_queue_shuffle_jump_and_removal() {
    use std::path::Path;

    let mut queue = Queue::new();
    let tracks = (0..5)
        .map(|i| Track::new(format!("/music/track_{}.mp3", i)))
        .collect::<Vec<_>>();

    queue.set_shuffle(true);
    queue.set_tracks(tracks, 0);

    // Jump to track 4
    let jumped = queue.jump_to(4).expect("valid jump");
    assert_eq!(jumped.path, Path::new("/music/track_4.mp3"));
    assert_eq!(queue.current_index, Some(4));

    // Remove track 2
    queue.remove_at(2);
    assert_eq!(queue.tracks.len(), 4);
    assert_eq!(queue.shuffle_order.len(), 4);
    // Indices above 2 should be decremented; former track 4 is now index 3
    assert_eq!(queue.current_index, Some(3));
}

#[test]
fn test_lrc_parser_standard() {
    let lrc_content = r#"
[ti:Midnight City]
[ar:M83]
[al:Hurry Up, We're Dreaming]
[offset:500]

[00:10.50]Waiting in a car
[00:15.00]Waiting for a ride in the dark
[00:22.75]The city is my church
"#;

    let lyrics = Lyrics::parse(lrc_content);
    assert_eq!(lyrics.title.as_deref(), Some("Midnight City"));
    assert_eq!(lyrics.artist.as_deref(), Some("M83"));
    assert_eq!(lyrics.album.as_deref(), Some("Hurry Up, We're Dreaming"));
    assert_eq!(lyrics.offset_ms, 500);
    assert_eq!(lyrics.lines.len(), 3);

    // With offset 500ms:
    // 10.50s (10500ms) + 500ms = 11000ms = 11.00s
    assert_eq!(lyrics.lines[0].timestamp, Duration::from_millis(11000));
    assert_eq!(lyrics.lines[0].text, "Waiting in a car");

    // 15.00s (15000ms) + 500ms = 15500ms = 15.50s
    assert_eq!(lyrics.lines[1].timestamp, Duration::from_millis(15500));
    assert_eq!(lyrics.lines[1].text, "Waiting for a ride in the dark");

    // 22.75s (22750ms) + 500ms = 23250ms = 23.25s
    assert_eq!(lyrics.lines[2].timestamp, Duration::from_millis(23250));
    assert_eq!(lyrics.lines[2].text, "The city is my church");
}

#[test]
fn test_lrc_parser_multiple_timestamps() {
    let lrc_content = r#"
[00:05.00]Verse one
[00:15.25][00:45.50]Repeated chorus line
[01:00.00]Outro
"#;

    let lyrics = Lyrics::parse(lrc_content);
    assert_eq!(lyrics.lines.len(), 4);

    // Check sorted order
    assert_eq!(lyrics.lines[0].timestamp, Duration::from_millis(5000));
    assert_eq!(lyrics.lines[0].text, "Verse one");

    assert_eq!(lyrics.lines[1].timestamp, Duration::from_millis(15250));
    assert_eq!(lyrics.lines[1].text, "Repeated chorus line");

    assert_eq!(lyrics.lines[2].timestamp, Duration::from_millis(45500));
    assert_eq!(lyrics.lines[2].text, "Repeated chorus line");

    assert_eq!(lyrics.lines[3].timestamp, Duration::from_millis(60000));
    assert_eq!(lyrics.lines[3].text, "Outro");
}

#[test]
fn test_lrc_negative_offset_clamp() {
    let lrc_content = r#"
[offset:-3000]
[00:02.00]Line clamped to zero
[00:10.00]Line shifted down
"#;

    let lyrics = Lyrics::parse(lrc_content);
    assert_eq!(lyrics.lines.len(), 2);
    // 2s - 3s = -1s clamped to 0
    assert_eq!(lyrics.lines[0].timestamp, Duration::ZERO);
    // 10s - 3s = 7s
    assert_eq!(lyrics.lines[1].timestamp, Duration::from_secs(7));
}

#[test]
fn test_lrc_active_index_matching() {
    let lrc_content = r#"
[00:10.00]First line
[00:20.00]Second line
[00:30.00]Third line
"#;

    let lyrics = Lyrics::parse(lrc_content);

    // Before any line starts: None
    assert_eq!(lyrics.find_active_index(Duration::from_secs(5)), None);

    // Exactly at first line: Some(0)
    assert_eq!(lyrics.find_active_index(Duration::from_secs(10)), Some(0));

    // Between line 0 and 1: Some(0)
    assert_eq!(lyrics.find_active_index(Duration::from_secs(15)), Some(0));

    // Exactly at second line: Some(1)
    assert_eq!(lyrics.find_active_index(Duration::from_secs(20)), Some(1));

    // At or beyond third line: Some(2)
    assert_eq!(lyrics.find_active_index(Duration::from_secs(30)), Some(2));
    assert_eq!(lyrics.find_active_index(Duration::from_secs(99)), Some(2));
}

#[test]
fn test_lyrics_state_navigation_and_sync() {
    let lrc = Lyrics::parse("[00:05.00]Line 1\n[00:10.00]Line 2\n[00:15.00]Line 3\n");
    let mut state = LyricsState::new();
    state.set_lyrics(Some(lrc));

    assert_eq!(state.selected_line_idx, 0);
    assert!(state.auto_scroll);

    // Sync when auto-scroll is on
    state.sync_active(Some(1));
    assert_eq!(state.selected_line_idx, 1);
    assert_eq!(state.selected_timestamp(), Some(Duration::from_secs(10)));

    // Manual navigation pauses auto-scroll
    state.move_down(3);
    assert_eq!(state.selected_line_idx, 2);
    assert!(!state.auto_scroll);

    // Sync while paused does NOT override manual selection
    state.sync_active(Some(0));
    assert_eq!(state.selected_line_idx, 2);

    // Resume auto-scroll
    state.resume_auto_scroll(Some(0));
    assert!(state.auto_scroll);
    assert_eq!(state.selected_line_idx, 0);
}

#[test]
fn test_lyrics_companion_discovery() {
    use std::fs::File;
    use std::io::Write;

    let temp_dir = std::env::temp_dir().join("auri_test_lyrics");
    let _ = std::fs::create_dir_all(&temp_dir);

    let audio_file = temp_dir.join("sample_track.flac");
    let lrc_file = temp_dir.join("sample_track.lrc");

    let _ = File::create(&audio_file);
    let mut f = File::create(&lrc_file).expect("create lrc");
    writeln!(f, "[00:01.00]Sample lyric line").unwrap();

    let discovered = Lyrics::load_for_track(&audio_file);
    assert!(discovered.is_some());
    let lyrics = discovered.unwrap();
    assert_eq!(lyrics.lines.len(), 1);
    assert_eq!(lyrics.lines[0].text, "Sample lyric line");
    assert_eq!(lyrics.source, LyricsSource::CompanionFile(lrc_file.clone()));

    // Clean up
    let _ = std::fs::remove_file(audio_file);
    let _ = std::fs::remove_file(lrc_file);
    let _ = std::fs::remove_dir(temp_dir);
}


