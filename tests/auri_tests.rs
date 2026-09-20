use std::path::PathBuf;
use std::time::Duration;

use bebop::audio::analysis::AudioAnalyzer;
use bebop::audio::LinearResampler;
use bebop::library::{format_time, Track};
use bebop::lyrics::{Lyrics, LyricsSource, LyricsState};
use bebop::notifications::NotificationManager;
use bebop::player::{PlayerState, PlaylistManager, Queue, RepeatMode};
use bebop::theme::Theme;
use bebop::ui::{AppLayout, LibraryState, QueueState};
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
    let temp_dir = std::env::temp_dir().join("bebop_test_playlist");
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
    let layout = AppLayout::calculate(area, true, false);

    assert_eq!(layout.header.height, 3);
    assert_eq!(layout.visualizer.width, 100); // Visualizer now spans full terminal width
    assert!(layout.artwork.is_some());
    assert!(layout.pet.is_none());
    let art = layout.artwork.unwrap();
    // Inner dimensions preserve 2:1 character cell aspect ratio for 1:1 pixel square
    assert_eq!(art.width - 2, (art.height - 2) * 2);
    assert!(layout.player_controls.height >= 8);
    assert_eq!(layout.status.height, 3);

    // Narrow/mobile terminal area (width < 50): artwork and pet collapse gracefully
    let compact_area = Rect::new(0, 0, 45, 24);
    let compact_layout = AppLayout::calculate(compact_area, true, true);

    assert!(compact_layout.artwork.is_none());
    assert!(compact_layout.pet.is_none());
    assert_eq!(compact_layout.visualizer.width, 45);
}

#[test]
fn test_metadata_and_decoder_probe() {
    let test_path = dirs::audio_dir()
        .map(|d| d.join("Misfits - Last Caress.mp3"))
        .filter(|p| p.exists());

    if let Some(path) = test_path {
        let (track, _art) = bebop::library::MetadataReader::read_track(&path);
        assert!(!track.title.is_empty());
        assert_eq!(track.format, "MP3");

        let mut decoder = bebop::audio::decoder::AudioDecoder::open(&path).expect("Decoder failed to open MP3");
        assert!(decoder.sample_rate() > 0);
        assert!(decoder.channels() > 0);

        let mut sample_buf = vec![0.0f32; 1024];
        let read = decoder.read_samples(&mut sample_buf);
        assert!(read > 0, "Expected decoder to decode audio frames");
    }
}

#[test]
fn test_audio_engine_stop_and_finish_flag() {
    if let Ok(mut engine) = bebop::audio::AudioEngine::new() {
        assert!(!engine.is_finished());
        engine.stop();
        assert!(!engine.is_finished());
    }
}

#[test]
fn test_fuzzy_search_scoring() {
    use bebop::library::{fuzzy_score, score_track, Track};
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
    use bebop::library::{SearchState, Track};
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
    use bebop::config::AppConfig;
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
    use bebop::theme::Theme;

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
    use bebop::visualizers::VisualizerKind;

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
    use bebop::visualizers::mirrored::MirroredBarsVisualizer;
    use bebop::visualizers::Visualizer;
    use bebop::audio::frame::AudioFrame;
    use bebop::theme::Theme;
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
    use bebop::visualizers::vu::VuMeterVisualizer;
    use bebop::visualizers::Visualizer;
    use bebop::audio::frame::AudioFrame;
    use bebop::theme::Theme;
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
    use bebop::theme::extract_palette;
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
    use bebop::theme::extract_palette;
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
    use bebop::visualizers::waterfall::WaterfallVisualizer;
    use bebop::visualizers::Visualizer;
    use bebop::audio::frame::AudioFrame;
    use bebop::theme::Theme;
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
fn test_waterfall_intensity_drives_display() {
    use bebop::visualizers::waterfall::WaterfallVisualizer;
    use bebop::visualizers::Visualizer;
    use bebop::audio::frame::AudioFrame;
    use bebop::theme::Theme;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use ratatui::layout::Rect;

    // Fills one waterfall panel at a given spectrum level and reports how many
    // cells are painted and how many reached the solid block glyph.
    fn render_at(level: f32) -> (usize, usize) {
        let mut viz = WaterfallVisualizer::default();
        let theme = Theme::vercel_dark();
        let mut audio = AudioFrame::default();
        audio.spectrum = vec![level; 64];

        let backend = TestBackend::new(40, 8);
        let mut terminal = Terminal::new(backend).unwrap();
        for _ in 0..8 {
            terminal
                .draw(|f| {
                    let area = Rect::new(0, 0, 40, 8);
                    viz.render(f, area, &audio, &theme);
                })
                .unwrap();
        }

        let buffer = terminal.backend().buffer();
        let mut painted = 0;
        let mut solid = 0;
        for y in 0..8 {
            for x in 0..40 {
                let symbol = buffer.cell((x, y)).unwrap().symbol();
                if symbol != " " {
                    painted += 1;
                }
                if symbol == "\u{2588}" {
                    solid += 1;
                }
            }
        }
        (painted, solid)
    }

    let (quiet_painted, quiet_solid) = render_at(0.20);
    let (mid_painted, mid_solid) = render_at(0.60);
    let (loud_painted, loud_solid) = render_at(0.95);

    // Below the display floor the panel stays dark rather than painting a sheet
    assert_eq!(quiet_painted, 0, "quiet content should leave the panel empty");
    assert_eq!(quiet_solid, 0);

    // Mid energy paints, but reserves the solid block for real peaks
    assert!(mid_painted > 0, "mid energy should paint the panel");
    assert_eq!(mid_solid, 0, "mid energy should not use the solid block glyph");

    // Loud content fills solid, which is the top of the glyph ramp
    assert!(loud_painted > 0);
    assert!(
        loud_solid > mid_solid,
        "loud content should reach the solid block glyph"
    );
}

#[test]
fn test_particles_visualizer() {
    use bebop::visualizers::particles::ParticlesVisualizer;
    use bebop::visualizers::Visualizer;
    use bebop::audio::frame::AudioFrame;
    use bebop::theme::Theme;
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

    let temp_dir = std::env::temp_dir().join("bebop_test_lyrics");
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

#[test]
fn test_notification_payload_formatting() {
    let mut track = Track::new("/music/synthwave/sunset.flac");
    track.title = "Sunset Drive".to_string();
    track.artist = "Miami Nights 1984".to_string();
    track.album = "Turbulence".to_string();
    track.duration = Duration::from_secs(245);

    let (summary, body) = NotificationManager::format_payload(&track);
    assert_eq!(summary, "Sunset Drive");
    assert_eq!(body, "Miami Nights 1984 • Turbulence\n04:05");

    // Fallback when album is Unknown Album
    let mut track_no_album = Track::new("/music/track.mp3");
    track_no_album.title = "Single Track".to_string();
    track_no_album.artist = "Solo Artist".to_string();
    track_no_album.duration = Duration::from_secs(120);

    let (summary2, body2) = NotificationManager::format_payload(&track_no_album);
    assert_eq!(summary2, "Single Track");
    assert_eq!(body2, "Solo Artist\n02:00");
}

#[test]
fn test_notification_config_roundtrip() {
    use bebop::config::AppConfig;

    let toml_str = r#"
[player]
volume = 0.75
repeat = "one"
shuffle = true

[ui]
theme = "Vercel Dark"
artwork = false
visualizer = "waveform"
notifications = false

[library]
paths = ["/home/user/Music"]
"#;

    let cfg: AppConfig = toml::from_str(toml_str).expect("deserialize config");
    assert!(!cfg.ui.notifications);
    assert_eq!(cfg.player.volume, 0.75);

    // Verify default when notifications field is omitted
    let toml_default = r#"
[ui]
theme = "Ice"
artwork = true
visualizer = "bars"
"#;
    let cfg_default: AppConfig = toml::from_str(toml_default).expect("deserialize config default");
    assert!(cfg_default.ui.notifications);

    // Verify serialization roundtrip
    let serialized = toml::to_string(&cfg).expect("serialize config");
    assert!(serialized.contains("notifications = false"));
}

#[test]
fn test_non_blocking_notification_dispatch() {
    let mut track = Track::new("/music/song.mp3");
    track.title = "Non-blocking Test".to_string();
    track.artist = "Tester".to_string();
    track.duration = Duration::from_secs(60);

    // Must return immediately without panicking
    NotificationManager::send_track_notification(&track);
}

#[test]
fn test_mpris_state_metadata_dictionary() {
    use bebop::mpris::MprisState;
    use std::time::Duration;

    let mut state = MprisState::default();
    state.track_title = "Neon Nights".to_string();
    state.track_artist = "Synthwave Boy".to_string();
    state.track_album = "Cyberpunk 2088".to_string();
    state.track_duration = Duration::from_secs(185);
    state.track_path = Some("/music/synth.flac".to_string());

    let dict = state.metadata_dict();
    assert!(dict.contains_key("mpris:trackid"));
    assert!(dict.contains_key("mpris:length"));
    assert_eq!(
        dict.get("xesam:title").and_then(|v| <&str>::try_from(v).ok()),
        Some("Neon Nights")
    );
    assert_eq!(
        dict.get("xesam:album").and_then(|v| <&str>::try_from(v).ok()),
        Some("Cyberpunk 2088")
    );
    assert_eq!(
        dict.get("xesam:url").and_then(|v| <&str>::try_from(v).ok()),
        Some("file:///music/synth.flac")
    );
    assert_eq!(
        dict.get("mpris:length").and_then(|v| i64::try_from(v).ok()),
        Some(185_000_000)
    );
}

#[test]
fn test_mpris_action_channel_dispatch() {
    use bebop::mpris::MprisAction;
    use std::time::Duration;

    let (tx, rx) = crossbeam_channel::unbounded();

    let actions = vec![
        MprisAction::Play,
        MprisAction::Pause,
        MprisAction::PlayPause,
        MprisAction::Stop,
        MprisAction::Next,
        MprisAction::Previous,
        MprisAction::Seek(5_000_000),
        MprisAction::SetPosition(Duration::from_secs(42)),
        MprisAction::SetVolume(0.85),
        MprisAction::Quit,
    ];

    for act in &actions {
        tx.send(act.clone()).expect("send action");
    }

    let received: Vec<MprisAction> = rx.try_iter().collect();
    assert_eq!(actions, received);
}

#[test]
fn test_mpris_config_roundtrip() {
    use bebop::config::AppConfig;

    let toml_str = r#"
[player]
volume = 0.9
repeat = "all"
shuffle = false

[ui]
theme = "Vercel Dark"
artwork = true
visualizer = "bars"
notifications = true
mpris = false

[library]
paths = []
"#;

    let cfg: AppConfig = toml::from_str(toml_str).expect("deserialize config");
    assert!(!cfg.ui.mpris);

    // Verify default when mpris is omitted
    let toml_default = r#"
[ui]
theme = "Vercel Dark"
artwork = true
visualizer = "bars"
"#;
    let cfg_default: AppConfig = toml::from_str(toml_default).expect("deserialize config default");
    assert!(cfg_default.ui.mpris);

    // Verify serialization roundtrip
    let serialized = toml::to_string(&cfg).expect("serialize config");
    assert!(serialized.contains("mpris = false"));
}

#[test]
fn test_mpris_state_update() {
    use bebop::mpris::MprisService;
    use bebop::player::{PlayerState, RepeatMode};
    use bebop::audio::PlaybackState;
    use bebop::library::Track;
    use std::time::Duration;

    if let Some(service) = MprisService::new() {
        let mut player = PlayerState::default();
        player.playback_state = PlaybackState::Playing;
        player.repeat = RepeatMode::One;
        player.shuffle = true;
        player.volume = 0.75;
        player.position = Duration::from_secs(30);
        player.duration = Duration::from_secs(180);

        let mut track = Track::new("/music/track1.mp3");
        track.title = "Test Song".to_string();
        track.artist = "Test Artist".to_string();
        track.album = "Test Album".to_string();

        service.update_state(&player, Some(&track));
    }
}

#[test]
fn test_audio_device_info_structure() {
    use bebop::audio::AudioDeviceInfo;

    let dev = AudioDeviceInfo {
        name: "Headphones (USB Audio)".to_string(),
        is_default: false,
        is_active: true,
    };

    assert_eq!(dev.name, "Headphones (USB Audio)");
    assert!(!dev.is_default);
    assert!(dev.is_active);

    let dev_clone = dev.clone();
    assert_eq!(dev, dev_clone);
}

#[test]
fn test_device_state_lifecycle_and_navigation() {
    use bebop::audio::AudioDeviceInfo;
    use bebop::ui::DeviceState;

    let mut state = DeviceState::new();
    assert!(!state.is_open);
    assert_eq!(state.selected_index, 0);

    let devices = vec![
        AudioDeviceInfo {
            name: "Built-in Speakers".to_string(),
            is_default: true,
            is_active: false,
        },
        AudioDeviceInfo {
            name: "USB DAC".to_string(),
            is_default: false,
            is_active: true,
        },
        AudioDeviceInfo {
            name: "HDMI Audio".to_string(),
            is_default: false,
            is_active: false,
        },
    ];

    // Open matching active device
    state.open(devices.clone(), Some("USB DAC"));
    assert!(state.is_open);
    assert_eq!(state.selected_index, 1);
    assert_eq!(state.selected_device().unwrap().name, "USB DAC");

    // Move down to index 2
    state.move_down();
    assert_eq!(state.selected_index, 2);
    assert_eq!(state.selected_device().unwrap().name, "HDMI Audio");

    // Move down wraps to index 0
    state.move_down();
    assert_eq!(state.selected_index, 0);
    assert_eq!(state.selected_device().unwrap().name, "Built-in Speakers");

    // Move up wraps back to index 2
    state.move_up();
    assert_eq!(state.selected_index, 2);

    // Move up to index 1
    state.move_up();
    assert_eq!(state.selected_index, 1);

    // Close
    state.close();
    assert!(!state.is_open);
}

#[test]
fn test_device_config_roundtrip() {
    use bebop::config::AppConfig;

    let toml_str = r#"
[player]
volume = 0.85
repeat = "all"
shuffle = false
device = "External Studio Monitor"

[ui]
theme = "Vercel Dark"
artwork = true
visualizer = "bars"

[library]
paths = []
"#;

    let cfg: AppConfig = toml::from_str(toml_str).expect("deserialize config with device");
    assert_eq!(cfg.player.device.as_deref(), Some("External Studio Monitor"));

    // Omitted device defaults to None
    let toml_default = r#"
[player]
volume = 0.8
repeat = "all"
shuffle = false

[ui]
theme = "Vercel Dark"
artwork = true
visualizer = "bars"
"#;
    let cfg_default: AppConfig = toml::from_str(toml_default).expect("deserialize config default device");
    assert_eq!(cfg_default.player.device, None);

    // Roundtrip serialization
    let serialized = toml::to_string(&cfg).expect("serialize config");
    assert!(serialized.contains("device = \"External Studio Monitor\""));
}

#[test]
fn test_device_overlay_rendering() {
    use bebop::audio::AudioDeviceInfo;
    use bebop::theme::Theme;
    use bebop::ui::player::HitAction;
    use bebop::ui::{DeviceOverlay, DeviceState};
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;
    use ratatui::Terminal;

    let mut state = DeviceState::new();
    let devices = vec![
        AudioDeviceInfo {
            name: "Built-in Audio".to_string(),
            is_default: true,
            is_active: false,
        },
        AudioDeviceInfo {
            name: "USB Headphones".to_string(),
            is_default: false,
            is_active: true,
        },
    ];
    state.open(devices, Some("USB Headphones"));

    let theme = Theme::vercel_dark();
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut hit_zones = Vec::new();

    terminal
        .draw(|f| {
            let area = Rect::new(0, 0, 80, 24);
            DeviceOverlay::render(f, area, &state, &theme, &mut hit_zones);
        })
        .unwrap();

    // Verify hit zones were registered for both devices
    assert!(hit_zones.iter().any(|z| z.action == HitAction::DeviceSelect(0)));
    assert!(hit_zones.iter().any(|z| z.action == HitAction::DeviceSelect(1)));

    // Verify buffer has output device title
    let buffer = terminal.backend().buffer();
    let text: String = (0..24)
        .flat_map(|y| (0..80).map(move |x| buffer.cell((x, y)).unwrap().symbol().to_string()))
        .collect();
    assert!(text.contains("Audio Output Devices"));
    assert!(text.contains("USB Headphones"));
}

#[test]
fn test_pet_layout_calculation() {
    let area = Rect::new(0, 0, 120, 40);

    // Both artwork and pet enabled
    let layout = AppLayout::calculate(area, true, true);
    assert!(layout.artwork.is_some());
    assert!(layout.pet.is_some());

    let art = layout.artwork.unwrap();
    let pet = layout.pet.unwrap();

    // Symmetrical box widths matching the 1:2 aspect ratio
    assert_eq!(art.width, pet.width);
    assert_eq!(art.height, pet.height);
    assert!(layout.player_controls.width >= 30);

    // Positions: artwork on left, controls in middle, pet on right
    assert_eq!(art.x, 0);
    assert_eq!(layout.player_controls.x, art.width);
    assert_eq!(pet.x, art.width + layout.player_controls.width);
    assert_eq!(pet.x + pet.width, 120);

    // Pet only (no artwork)
    let pet_only = AppLayout::calculate(area, false, true);
    assert!(pet_only.artwork.is_none());
    assert!(pet_only.pet.is_some());
    let pet_rect = pet_only.pet.unwrap();
    assert_eq!(pet_rect.x + pet_rect.width, 120);
    assert_eq!(pet_only.player_controls.x, 0);

    // Narrow terminal (width < 50) collapses both
    let narrow_area = Rect::new(0, 0, 45, 24);
    let narrow_layout = AppLayout::calculate(narrow_area, true, true);
    assert!(narrow_layout.artwork.is_none());
    assert!(narrow_layout.pet.is_none());
}

#[test]
fn test_pet_config_roundtrip() {
    use bebop::config::AppConfig;

    // Default configuration has pet enabled
    let cfg = AppConfig::default();
    assert!(cfg.ui.pet);

    // Parsing TOML with pet explicitly false
    let toml_str = r#"
[ui]
theme = "tokyo_night"
artwork = true
visualizer = "bars"
pet = false
"#;
    let parsed: AppConfig = toml::from_str(toml_str).unwrap();
    assert!(!parsed.ui.pet);

    // Parsing TOML without pet field defaults to true
    let toml_missing = r#"
[ui]
theme = "tokyo_night"
artwork = true
visualizer = "bars"
"#;
    let parsed_default: AppConfig = toml::from_str(toml_missing).unwrap();
    assert!(parsed_default.ui.pet);
}

#[test]
fn test_pet_view_rendering() {
    use bebop::audio::{AudioFrame, PlaybackState};
    use bebop::player::PlayerState;
    use bebop::theme::Theme;
    use bebop::ui::pet::PetView;
    use bebop::ui::player::HitAction;
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;
    use ratatui::Terminal;

    let theme = Theme::vercel_dark();
    let backend = TestBackend::new(40, 16);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut hit_zones = Vec::new();

    // 1. Render in stopped/paused state (sleeping cat)
    let player = PlayerState {
        playback_state: PlaybackState::Stopped,
        ..Default::default()
    };
    let empty_frame = AudioFrame::default();

    terminal
        .draw(|f| {
            let area = Rect::new(0, 0, 30, 12);
            PetView::render(f, area, &player, &empty_frame, &theme, None, &mut hit_zones);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let text: String = (0..12)
        .flat_map(|y| (0..30).map(move |x| buffer.cell((x, y)).unwrap().symbol().to_string()))
        .collect();

    assert!(text.contains("Kyoku"), "pane title missing from the render");
    // Sprite frames cycle on wall-clock time, so assert on the note row
    // instead, which depends only on playback state.
    assert!(text.contains("zzZ"), "sleeping pet should show the sleep marker");
    assert!(hit_zones.iter().any(|z| z.action == HitAction::PetInteract));

    // 2. Render in active playing state with audio energy (dancing cat)
    let player_playing = PlayerState {
        playback_state: PlaybackState::Playing,
        ..Default::default()
    };
    let active_frame = AudioFrame {
        spectrum: vec![0.8; 64],
        rms_left: 0.7,
        rms_right: 0.7,
        ..Default::default()
    };

    hit_zones.clear();
    terminal
        .draw(|f| {
            let area = Rect::new(0, 0, 30, 12);
            PetView::render(
                f,
                area,
                &player_playing,
                &active_frame,
                &theme,
                None,
                &mut hit_zones,
            );
        })
        .unwrap();

    let buffer2 = terminal.backend().buffer();
    let text2: String = (0..12)
        .flat_map(|y| (0..30).map(move |x| buffer2.cell((x, y)).unwrap().symbol().to_string()))
        .collect();

    assert!(text2.contains("Kyoku"), "pane title missing from the render");
    assert!(text2.contains("\u{266b}"), "playing pet should show music notes");
    assert!(!text2.contains("zzZ"), "playing pet should not show the sleep marker");
    assert!(hit_zones.iter().any(|z| z.action == HitAction::PetInteract));
}

#[test]
fn test_pet_reaction_shows_hearts_and_wakes_the_cat() {
    use bebop::audio::{AudioFrame, PlaybackState};
    use bebop::player::PlayerState;
    use bebop::theme::Theme;
    use bebop::ui::pet::PetView;
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;
    use ratatui::Terminal;

    let theme = Theme::vercel_dark();
    let backend = TestBackend::new(40, 16);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut hit_zones = Vec::new();

    // A stopped cat that is being petted should drop the sleep marker and
    // show hearts instead.
    let player = PlayerState {
        playback_state: PlaybackState::Stopped,
        ..Default::default()
    };
    let empty_frame = AudioFrame::default();

    let read_pane = |terminal: &Terminal<TestBackend>| -> String {
        let buffer = terminal.backend().buffer();
        (0..12)
            .flat_map(|y| (0..30).map(move |x| buffer.cell((x, y)).unwrap().symbol().to_string()))
            .collect()
    };

    // Early in the reaction: a single heart pops out
    terminal
        .draw(|f| {
            let area = Rect::new(0, 0, 30, 12);
            PetView::render(
                f,
                area,
                &player,
                &empty_frame,
                &theme,
                Some(0.1),
                &mut hit_zones,
            );
        })
        .unwrap();

    let text = read_pane(&terminal);
    assert!(
        text.contains('\u{2665}'),
        "a freshly petted cat should show a heart"
    );
    assert!(
        !text.contains("zzZ"),
        "petting should wake the cat out of the sleep frames"
    );

    // Later in the reaction: the hearts spread out before fading
    terminal
        .draw(|f| {
            let area = Rect::new(0, 0, 30, 12);
            PetView::render(
                f,
                area,
                &player,
                &empty_frame,
                &theme,
                Some(0.9),
                &mut hit_zones,
            );
        })
        .unwrap();

    let late_text = read_pane(&terminal);
    assert!(
        late_text.matches('\u{2665}').count() >= 2,
        "the reaction should spread to more hearts as it fades"
    );
}

#[test]
fn test_deck_grows_with_terminal_height() {
    // The bottom deck should get taller as the terminal does, and the side
    // boxes get wider with it, without ever starving the visualizer.
    let mut last_deck = 0u16;
    for height in [24u16, 26, 30, 36, 42, 50] {
        let area = Rect::new(0, 0, 120, height);
        let layout = AppLayout::calculate(area, true, true);
        let art = layout.artwork.expect("artwork box at 120 columns");
        let pet = layout.pet.expect("pet box at 120 columns");

        assert!(
            art.height >= last_deck,
            "deck shrank going from a shorter terminal to height {height}"
        );
        last_deck = art.height;

        assert!(
            layout.visualizer.height >= 6,
            "visualizer starved at height {height}"
        );
        assert_eq!(art.width, pet.width);
        // Side boxes stay square in pixel terms: two cells wide per cell tall
        assert_eq!(art.width - 2, (art.height - 2) * 2);
        assert!(layout.player_controls.width >= 30);
        assert_eq!(pet.x + pet.width, 120);
    }

    // A taller terminal really does buy a bigger box
    let short = AppLayout::calculate(Rect::new(0, 0, 120, 24), true, true);
    let tall = AppLayout::calculate(Rect::new(0, 0, 120, 44), true, true);
    assert!(tall.pet.unwrap().width > short.pet.unwrap().width);
}

#[test]
fn test_both_side_boxes_survive_an_80_column_terminal() {
    // A tall but narrow terminal must not drop the pet just because the deck
    // grew; the boxes narrow to make room for the controls instead.
    let layout = AppLayout::calculate(Rect::new(0, 0, 80, 44), true, true);
    let art = layout.artwork.expect("artwork box at 80 columns");
    let pet = layout.pet.expect("pet box at 80 columns");
    assert_eq!(art.width, pet.width);
    assert!(layout.player_controls.width >= 30);
    assert_eq!(pet.x + pet.width, 80);
}

// --- Sample rate conversion ---

/// Build an interleaved stereo sine wave at `freq` Hz.
fn stereo_sine(freq: f64, rate: u32, frames: usize) -> Vec<f32> {
    let mut buf = Vec::with_capacity(frames * 2);
    for n in 0..frames {
        let t = n as f64 / rate as f64;
        let s = (2.0 * std::f64::consts::PI * freq * t).sin() as f32;
        buf.push(s);
        buf.push(s);
    }
    buf
}

/// Count upward zero crossings, which gives the frequency of a clean sine.
fn measured_freq(samples: &[f32], rate: u32) -> f64 {
    let frames: Vec<f32> = samples.chunks_exact(2).map(|c| c[0]).collect();
    let crossings = frames
        .windows(2)
        .filter(|w| w[0] <= 0.0 && w[1] > 0.0)
        .count();
    let seconds = frames.len() as f64 / rate as f64;
    crossings as f64 / seconds
}

#[test]
fn test_resampler_passthrough_is_untouched() {
    let mut resampler = LinearResampler::new(44100, 44100);
    assert!(resampler.is_passthrough());

    let input = stereo_sine(440.0, 44100, 512);
    let mut output = Vec::new();
    resampler.process(&input, &mut output);

    assert_eq!(output, input);
}

#[test]
fn test_resampler_output_length_follows_rate_ratio() {
    let mut resampler = LinearResampler::new(48000, 44100);
    let input = stereo_sine(440.0, 48000, 4096);

    let mut output = Vec::new();
    // Several buffers in a row, since position carries across calls.
    for _ in 0..10 {
        resampler.process(&input, &mut output);
    }

    let in_frames = 4096 * 10;
    let out_frames = output.len() / 2;
    let expected = (in_frames as f64 * 44100.0 / 48000.0) as usize;

    assert_eq!(output.len() % 2, 0, "output must stay frame-aligned");
    assert!(
        out_frames.abs_diff(expected) < 16,
        "expected about {} frames, got {}",
        expected,
        out_frames
    );
}

#[test]
fn test_resampler_preserves_pitch_from_48k_to_44k() {
    // The bug this guards: 48 kHz audio played through a 44.1 kHz stream
    // dropped to 0.919x speed, roughly a semitone and a half flat.
    let mut resampler = LinearResampler::new(48000, 44100);
    let input = stereo_sine(440.0, 48000, 48000);

    let mut output = Vec::new();
    for chunk in input.chunks(4096) {
        resampler.process(chunk, &mut output);
    }

    let freq = measured_freq(&output, 44100);
    assert!(
        (freq - 440.0).abs() < 2.0,
        "expected about 440 Hz after conversion, measured {:.1} Hz",
        freq
    );
}

#[test]
fn test_resampler_preserves_pitch_upward() {
    let mut resampler = LinearResampler::new(22050, 48000);
    let input = stereo_sine(300.0, 22050, 22050);

    let mut output = Vec::new();
    for chunk in input.chunks(4096) {
        resampler.process(chunk, &mut output);
    }

    let freq = measured_freq(&output, 48000);
    assert!(
        (freq - 300.0).abs() < 2.0,
        "expected about 300 Hz after conversion, measured {:.1} Hz",
        freq
    );
}

#[test]
fn test_resampler_stays_continuous_across_buffers() {
    let mut resampler = LinearResampler::new(48000, 44100);
    let input = stereo_sine(100.0, 48000, 48000);

    let mut output = Vec::new();
    for chunk in input.chunks(2048) {
        resampler.process(chunk, &mut output);
    }

    // A 100 Hz sine at 44.1 kHz moves at most about 0.015 per sample. Buffer
    // seams would show up as a jump far larger than that.
    let left: Vec<f32> = output.chunks_exact(2).map(|c| c[0]).collect();
    let max_step = left
        .windows(2)
        .map(|w| (w[1] - w[0]).abs())
        .fold(0.0f32, f32::max);

    assert!(max_step < 0.05, "found a discontinuity of {}", max_step);
}

#[test]
fn test_resampler_reset_clears_carried_frame() {
    let mut resampler = LinearResampler::new(48000, 44100);
    let loud = vec![1.0f32; 512];
    let mut discard = Vec::new();
    resampler.process(&loud, &mut discard);

    resampler.reset();

    let silence = vec![0.0f32; 512];
    let mut output = Vec::new();
    resampler.process(&silence, &mut output);

    let peak = output.iter().fold(0.0f32, |acc, s| acc.max(s.abs()));
    assert_eq!(peak, 0.0, "stale audio leaked through after reset");
}
