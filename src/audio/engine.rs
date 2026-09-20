use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::audio::analysis::AudioAnalyzer;
use crate::audio::decoder::AudioDecoder;
use crate::audio::frame::AudioFrame;
use crate::audio::output::{AudioOutput, OutputControls};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaybackState {
    Playing,
    Paused,
    Stopped,
}

struct EngineShared {
    ring_buffer: Mutex<VecDeque<f32>>,
    analysis_samples: Mutex<Vec<f32>>,
    current_time_ms: Mutex<u64>,
    duration_ms: Mutex<u64>,
    sample_rate: Mutex<u32>,
    channels: Mutex<usize>,
    state: Mutex<PlaybackState>,
    seek_target: Mutex<Option<Duration>>,
    track_finished: AtomicBool,
    generation: AtomicU64,
}

pub struct AudioEngine {
    shared: Arc<EngineShared>,
    _output: Option<AudioOutput>,
    analyzer: AudioAnalyzer,
    output_controls: Option<Arc<OutputControls>>,
    worker_handle: Option<JoinHandle<()>>,
    worker_stop: Option<Arc<AtomicBool>>,
    running: Arc<AtomicBool>,
    _current_path: Option<PathBuf>,
}

impl AudioEngine {
    pub fn new() -> Result<Self, anyhow::Error> {
        let shared = Arc::new(EngineShared {
            ring_buffer: Mutex::new(VecDeque::with_capacity(96000)),
            analysis_samples: Mutex::new(Vec::with_capacity(4096)),
            current_time_ms: Mutex::new(0),
            duration_ms: Mutex::new(0),
            sample_rate: Mutex::new(44100),
            channels: Mutex::new(2),
            state: Mutex::new(PlaybackState::Stopped),
            seek_target: Mutex::new(None),
            track_finished: AtomicBool::new(false),
            generation: AtomicU64::new(0),
        });

        let shared_output = Arc::clone(&shared);
        let sample_rate = 44100;

        // CPAL output pulling samples from shared ring buffer
        let output = AudioOutput::new(sample_rate, move |out: &mut [f32]| {
            let mut ring = shared_output.ring_buffer.lock().unwrap();
            let mut analysis = shared_output.analysis_samples.lock().unwrap();

            let to_read = out.len().min(ring.len());
            for i in 0..to_read {
                let sample = ring.pop_front().unwrap_or(0.0);
                out[i] = sample;
                if analysis.len() >= 4096 {
                    analysis.remove(0);
                }
                analysis.push(sample);
            }

            // Zero-fill if buffer underrun
            if to_read < out.len() {
                out[to_read..].fill(0.0);
            }
        }).ok();

        let output_controls = output.as_ref().map(|o| Arc::clone(&o.controls));
        let analyzer = AudioAnalyzer::new(sample_rate, 64);
        let running = Arc::new(AtomicBool::new(true));

        Ok(Self {
            shared,
            _output: output,
            analyzer,
            output_controls,
            worker_handle: None,
            worker_stop: None,
            running,
            _current_path: None,
        })
    }

    fn stop_worker(&mut self) {
        // Invalidate current generation so the running worker immediately ignores shared state
        self.shared.generation.fetch_add(1, Ordering::SeqCst);

        // Signal worker cancellation
        if let Some(stop) = self.worker_stop.take() {
            stop.store(true, Ordering::SeqCst);
        }

        // Wait for worker to finish cleanly
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
    }

    pub fn play_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), anyhow::Error> {
        let path_buf = path.as_ref().to_path_buf();
        self.stop();

        let mut decoder = AudioDecoder::open(&path_buf)?;
        let sample_rate = decoder.sample_rate();
        let channels = decoder.channels();
        let duration = decoder.duration();

        let current_gen = self.shared.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let worker_stop = Arc::new(AtomicBool::new(false));
        self.worker_stop = Some(Arc::clone(&worker_stop));

        *self.shared.sample_rate.lock().unwrap() = sample_rate;
        *self.shared.channels.lock().unwrap() = channels;
        *self.shared.duration_ms.lock().unwrap() = duration.as_millis() as u64;
        *self.shared.current_time_ms.lock().unwrap() = 0;
        self.shared.ring_buffer.lock().unwrap().clear();
        self.shared.analysis_samples.lock().unwrap().clear();
        self.shared.track_finished.store(false, Ordering::SeqCst);
        *self.shared.state.lock().unwrap() = PlaybackState::Playing;

        if let Some(controls) = &self.output_controls {
            controls.set_paused(false);
        }

        self._current_path = Some(path_buf);
        let shared = Arc::clone(&self.shared);
        let running = Arc::clone(&self.running);

        let handle = thread::spawn(move || {
            let mut read_buf = vec![0.0f32; 4096];

            while running.load(Ordering::Relaxed) && !worker_stop.load(Ordering::Relaxed) {
                if shared.generation.load(Ordering::Relaxed) != current_gen {
                    break;
                }

                // Check if playback was stopped
                let state = *shared.state.lock().unwrap();
                if state == PlaybackState::Stopped {
                    break;
                }

                // Check for seek request
                let seek_req = {
                    let mut seek_lock = shared.seek_target.lock().unwrap();
                    seek_lock.take()
                };

                if let Some(target) = seek_req {
                    let _ = decoder.seek(target);
                    if shared.generation.load(Ordering::Relaxed) != current_gen || worker_stop.load(Ordering::Relaxed) {
                        break;
                    }
                    shared.ring_buffer.lock().unwrap().clear();
                    *shared.current_time_ms.lock().unwrap() = target.as_millis() as u64;
                }

                if state == PlaybackState::Paused {
                    thread::sleep(Duration::from_millis(15));
                    continue;
                }

                // Keep ring buffer fed with roughly 1 second of audio
                let buffered_len = shared.ring_buffer.lock().unwrap().len();
                if buffered_len >= 88200 {
                    thread::sleep(Duration::from_millis(15));
                    continue;
                }

                let frames_read = decoder.read_samples(&mut read_buf);
                if shared.generation.load(Ordering::Relaxed) != current_gen || worker_stop.load(Ordering::Relaxed) {
                    break;
                }

                if frames_read == 0 {
                    // Check if ring buffer has emptied
                    if shared.ring_buffer.lock().unwrap().is_empty() {
                        if shared.generation.load(Ordering::Relaxed) == current_gen && !worker_stop.load(Ordering::Relaxed) {
                            shared.track_finished.store(true, Ordering::SeqCst);
                            *shared.state.lock().unwrap() = PlaybackState::Stopped;
                        }
                        break;
                    }
                    thread::sleep(Duration::from_millis(15));
                    continue;
                }

                if shared.generation.load(Ordering::Relaxed) != current_gen || worker_stop.load(Ordering::Relaxed) {
                    break;
                }

                let mut ring = shared.ring_buffer.lock().unwrap();
                ring.extend(&read_buf[..frames_read]);
                *shared.current_time_ms.lock().unwrap() = decoder.current_time().as_millis() as u64;
            }
        });

        self.worker_handle = Some(handle);
        Ok(())
    }

    pub fn pause(&self) {
        *self.shared.state.lock().unwrap() = PlaybackState::Paused;
        if let Some(controls) = &self.output_controls {
            controls.set_paused(true);
        }
    }

    pub fn resume(&self) {
        *self.shared.state.lock().unwrap() = PlaybackState::Playing;
        if let Some(controls) = &self.output_controls {
            controls.set_paused(false);
        }
    }

    pub fn toggle_play_pause(&self) -> PlaybackState {
        let current = *self.shared.state.lock().unwrap();
        match current {
            PlaybackState::Playing => {
                self.pause();
                PlaybackState::Paused
            }
            PlaybackState::Paused => {
                self.resume();
                PlaybackState::Playing
            }
            PlaybackState::Stopped => PlaybackState::Stopped,
        }
    }

    pub fn stop(&mut self) {
        self.stop_worker();
        *self.shared.state.lock().unwrap() = PlaybackState::Stopped;
        if let Some(controls) = &self.output_controls {
            controls.set_paused(true);
        }
        self.shared.ring_buffer.lock().unwrap().clear();
        self.shared.analysis_samples.lock().unwrap().clear();
        self.shared.track_finished.store(false, Ordering::SeqCst);
    }

    pub fn seek_to(&self, target: Duration) {
        *self.shared.seek_target.lock().unwrap() = Some(target);
    }

    pub fn seek_relative(&self, delta_secs: i64) {
        let current_ms = *self.shared.current_time_ms.lock().unwrap();
        let duration_ms = *self.shared.duration_ms.lock().unwrap();

        let new_ms = if delta_secs >= 0 {
            (current_ms + (delta_secs as u64 * 1000)).min(duration_ms)
        } else {
            current_ms.saturating_sub((-delta_secs) as u64 * 1000)
        };

        self.seek_to(Duration::from_millis(new_ms));
    }

    pub fn set_volume(&self, vol: f32) {
        if let Some(controls) = &self.output_controls {
            controls.set_volume(vol);
        }
    }

    pub fn volume(&self) -> f32 {
        self.output_controls.as_ref().map(|c| c.volume()).unwrap_or(0.8)
    }

    pub fn adjust_volume(&self, delta: f32) -> f32 {
        let current = self.volume();
        let new_vol = (current + delta).clamp(0.0, 1.0);
        self.set_volume(new_vol);
        new_vol
    }

    pub fn toggle_mute(&self) -> bool {
        self.output_controls.as_ref().map(|c| c.toggle_mute()).unwrap_or(false)
    }

    pub fn is_muted(&self) -> bool {
        self.output_controls.as_ref().map(|c| c.is_muted()).unwrap_or(false)
    }

    pub fn state(&self) -> PlaybackState {
        *self.shared.state.lock().unwrap()
    }

    pub fn current_position(&self) -> Duration {
        Duration::from_millis(*self.shared.current_time_ms.lock().unwrap())
    }

    pub fn duration(&self) -> Duration {
        Duration::from_millis(*self.shared.duration_ms.lock().unwrap())
    }

    pub fn is_finished(&self) -> bool {
        self.shared.track_finished.swap(false, Ordering::SeqCst)
    }

    /// Fetches the latest computed AudioFrame for visualizers.
    pub fn get_audio_frame(&mut self, num_bins: usize) -> AudioFrame {
        self.analyzer.set_num_bins(num_bins);
        let samples = {
            let analysis = self.shared.analysis_samples.lock().unwrap();
            analysis.clone()
        };
        self.analyzer.analyze(&samples)
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        self.stop();
        self.running.store(false, Ordering::Relaxed);
    }
}
