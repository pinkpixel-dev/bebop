use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::audio::analysis::AudioAnalyzer;
use crate::audio::decoder::AudioDecoder;
use crate::audio::frame::AudioFrame;
use crate::audio::output::{AudioOutput, OutputControls};
use crate::audio::resample::LinearResampler;

/// Fallback output rate used only when no device could be opened.
const DEFAULT_DEVICE_RATE: u32 = 44100;
/// Starting bin count for the analyzer. Visualizers override this per frame.
const ANALYZER_BINS: usize = 64;

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
    /// Sample rate of the active output stream. The decode worker resamples to
    /// this rate, and it changes when the user switches output device.
    device_rate: AtomicU32,
    state: Mutex<PlaybackState>,
    seek_target: Mutex<Option<Duration>>,
    track_finished: AtomicBool,
    generation: AtomicU64,
}

pub struct AudioEngine {
    shared: Arc<EngineShared>,
    output: Option<AudioOutput>,
    analyzer: AudioAnalyzer,
    device_rate: u32,
    output_controls: Option<Arc<OutputControls>>,
    worker_handle: Option<JoinHandle<()>>,
    worker_stop: Option<Arc<AtomicBool>>,
    running: Arc<AtomicBool>,
    _current_path: Option<PathBuf>,
    current_device_name: Option<String>,
}

impl AudioEngine {
    pub fn new() -> Result<Self, anyhow::Error> {
        Self::new_with_device(None)
    }

    pub fn new_with_device(preferred_device: Option<&str>) -> Result<Self, anyhow::Error> {
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
            device_rate: AtomicU32::new(DEFAULT_DEVICE_RATE),
        });

        let shared_output = Arc::clone(&shared);

        // CPAL output pulling samples from shared ring buffer
        let output = AudioOutput::new(preferred_device, move |out: &mut [f32]| {
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

        let current_device_name = output.as_ref().map(|o| o.device_name.clone());
        let output_controls = output.as_ref().map(|o| Arc::clone(&o.controls));
        let device_rate = output
            .as_ref()
            .map(|o| o.device_sample_rate)
            .unwrap_or(DEFAULT_DEVICE_RATE);
        shared.device_rate.store(device_rate, Ordering::SeqCst);
        let analyzer = AudioAnalyzer::new(device_rate, ANALYZER_BINS);
        let running = Arc::new(AtomicBool::new(true));

        Ok(Self {
            shared,
            output,
            analyzer,
            device_rate,
            output_controls,
            worker_handle: None,
            worker_stop: None,
            running,
            _current_path: None,
            current_device_name,
        })
    }

    /// Switch active audio output device at runtime without stopping decoder.
    pub fn switch_device(&mut self, device_name: Option<&str>) -> Result<(), anyhow::Error> {
        let vol = self.volume();
        let is_muted = self.is_muted();
        let is_paused = self.state() == PlaybackState::Paused;

        // Drop existing output stream
        self.output = None;
        self.output_controls = None;

        let shared_output = Arc::clone(&self.shared);

        let new_output = AudioOutput::new(device_name, move |out: &mut [f32]| {
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

            if to_read < out.len() {
                out[to_read..].fill(0.0);
            }
        })?;

        new_output.controls.set_volume(vol);
        if is_muted {
            new_output.controls.muted.store(true, Ordering::Relaxed);
        }
        if is_paused {
            new_output.controls.set_paused(true);
        }

        // The decode worker watches this and rebuilds its resampler, so a
        // device with a different rate does not change playback speed.
        let device_rate = new_output.device_sample_rate;
        if device_rate != self.device_rate {
            self.device_rate = device_rate;
            self.analyzer = AudioAnalyzer::new(device_rate, ANALYZER_BINS);
        }
        self.shared.device_rate.store(device_rate, Ordering::SeqCst);

        self.current_device_name = Some(new_output.device_name.clone());
        self.output_controls = Some(Arc::clone(&new_output.controls));
        self.output = Some(new_output);

        Ok(())
    }

    /// List all output devices reported by the host audio subsystem.
    pub fn available_devices(&self) -> Vec<crate::audio::output::AudioDeviceInfo> {
        crate::audio::output::list_output_devices(self.current_device_name.as_deref())
    }

    /// Current active output device name, if any.
    pub fn current_device_name(&self) -> Option<String> {
        self.current_device_name.clone()
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

        let device_rate = self.shared.device_rate.load(Ordering::SeqCst);

        let handle = thread::spawn(move || {
            let mut read_buf = vec![0.0f32; 4096];
            // Decoded audio comes out at the file's own rate, so it has to be
            // converted to the output stream's rate before it is queued.
            let mut resampler = LinearResampler::new(sample_rate, device_rate);
            let mut resampled = Vec::with_capacity(read_buf.len() * 2);

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
                    resampler.reset();
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

                // Picks up a rate change from switching output device mid-track.
                let active_rate = shared.device_rate.load(Ordering::Relaxed);
                if resampler.output_rate() != active_rate {
                    resampler = LinearResampler::new(sample_rate, active_rate);
                }

                resampled.clear();
                resampler.process(&read_buf[..frames_read], &mut resampled);

                let mut ring = shared.ring_buffer.lock().unwrap();
                ring.extend(resampled.iter().copied());
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
