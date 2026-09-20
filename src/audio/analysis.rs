use std::sync::Arc;
use rustfft::{Fft, FftPlanner, num_complex::Complex};
use crate::audio::frame::AudioFrame;

const FFT_SIZE: usize = 2048;
const WAVEFORM_SIZE: usize = 256;

/// Real-time audio analyzer transforming raw PCM samples into visualizer frames.
pub struct AudioAnalyzer {
    fft: Arc<dyn Fft<f32>>,
    sample_rate: u32,
    window: Vec<f32>,
    input_buffer: Vec<Complex<f32>>,
    output_buffer: Vec<Complex<f32>>,
    smoothed_spectrum: Vec<f32>,
    peak_caps: Vec<f32>,
    recent_waveform: Vec<f32>,
    num_bins: usize,
}

impl AudioAnalyzer {
    pub fn new(sample_rate: u32, num_bins: usize) -> Self {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);

        // Precompute Hann window to prevent spectral leakage
        let mut window = Vec::with_capacity(FFT_SIZE);
        for i in 0..FFT_SIZE {
            let val = 0.5 * (1.0 - ((2.0 * std::f32::consts::PI * i as f32) / (FFT_SIZE as f32 - 1.0)).cos());
            window.push(val);
        }

        Self {
            fft,
            sample_rate,
            window,
            input_buffer: vec![Complex::new(0.0, 0.0); FFT_SIZE],
            output_buffer: vec![Complex::new(0.0, 0.0); FFT_SIZE],
            smoothed_spectrum: vec![0.0; num_bins],
            peak_caps: vec![0.0; num_bins],
            recent_waveform: vec![0.0; WAVEFORM_SIZE],
            num_bins,
        }
    }

    pub fn set_num_bins(&mut self, bins: usize) {
        if bins != self.num_bins && bins > 0 {
            self.num_bins = bins;
            self.smoothed_spectrum.resize(bins, 0.0);
            self.peak_caps.resize(bins, 0.0);
        }
    }

    /// Process incoming stereo PCM samples (interleaved [L, R, L, R...])
    pub fn analyze(&mut self, samples: &[f32]) -> AudioFrame {
        if samples.is_empty() {
            // Decay smoothed values towards 0 when no audio is playing
            for val in self.smoothed_spectrum.iter_mut() {
                *val = (*val * 0.85).max(0.0);
            }
            for cap in self.peak_caps.iter_mut() {
                *cap = (*cap * 0.90).max(0.0);
            }
            return AudioFrame {
                spectrum: self.smoothed_spectrum.clone(),
                waveform: vec![0.0; WAVEFORM_SIZE],
                rms_left: 0.0,
                rms_right: 0.0,
                peak_left: 0.0,
                peak_right: 0.0,
            };
        }

        let num_frames = samples.len() / 2;
        let mut sum_sq_l = 0.0;
        let mut sum_sq_r = 0.0;
        let mut peak_l: f32 = 0.0;
        let mut peak_r: f32 = 0.0;

        // Calculate stereo metrics
        for chunk in samples.chunks_exact(2) {
            let l = chunk[0];
            let r = chunk[1];
            sum_sq_l += l * l;
            sum_sq_r += r * r;
            peak_l = peak_l.max(l.abs());
            peak_r = peak_r.max(r.abs());
        }

        let rms_l = if num_frames > 0 { (sum_sq_l / num_frames as f32).sqrt().min(1.0) } else { 0.0 };
        let rms_r = if num_frames > 0 { (sum_sq_r / num_frames as f32).sqrt().min(1.0) } else { 0.0 };

        // Fill recent waveform buffer with downsampled mono values
        let step = (num_frames / WAVEFORM_SIZE).max(1);
        self.recent_waveform.clear();
        for i in 0..WAVEFORM_SIZE {
            let idx = (i * step * 2).min(samples.len().saturating_sub(2));
            let mono = (samples[idx] + samples[idx + 1]) * 0.5;
            self.recent_waveform.push(mono);
        }

        // Fill FFT window from the tail end of the sample buffer
        let start_frame = num_frames.saturating_sub(FFT_SIZE);
        for i in 0..FFT_SIZE {
            let sample_idx = (start_frame + i) * 2;
            let mono = if sample_idx + 1 < samples.len() {
                (samples[sample_idx] + samples[sample_idx + 1]) * 0.5
            } else {
                0.0
            };
            let windowed = mono * self.window[i];
            self.input_buffer[i] = Complex::new(windowed, 0.0);
        }

        // Run FFT
        self.output_buffer.copy_from_slice(&self.input_buffer);
        self.fft.process(&mut self.output_buffer);

        // Group into logarithmic frequency bins (20 Hz to 20,000 Hz)
        let min_freq = 20.0f32;
        let max_freq = (self.sample_rate as f32 * 0.5).min(20000.0);
        let freq_ratio = (max_freq / min_freq).powf(1.0 / self.num_bins as f32);
        let bin_width_hz = self.sample_rate as f32 / FFT_SIZE as f32;

        let mut raw_bins = vec![0.0f32; self.num_bins];
        let mut current_freq = min_freq;

        for (b, bin_val) in raw_bins.iter_mut().enumerate() {
            let next_freq = current_freq * freq_ratio;
            let start_bin = ((current_freq / bin_width_hz).floor() as usize).min(FFT_SIZE / 2 - 1);
            let end_bin = ((next_freq / bin_width_hz).ceil() as usize).min(FFT_SIZE / 2);

            let mut max_mag = 0.0f32;
            for k in start_bin..=end_bin {
                let mag = self.output_buffer[k].norm() / (FFT_SIZE as f32 * 0.25);
                max_mag = max_mag.max(mag);
            }

            // High-frequency boost: human hearing is less sensitive at high freqs,
            // and music naturally rolls off at ~6dB/octave
            let freq_weight = 1.0 + (b as f32 / self.num_bins as f32).powf(1.4) * 3.5;
            *bin_val = (max_mag * freq_weight).min(1.0);
            current_freq = next_freq;
        }

        // Smoothing: fast attack (0.65) and smooth decay (0.2)
        for (i, &raw) in raw_bins.iter().enumerate() {
            let prev = self.smoothed_spectrum[i];
            if raw > prev {
                self.smoothed_spectrum[i] = prev + 0.65 * (raw - prev);
            } else {
                self.smoothed_spectrum[i] = prev - 0.20 * (prev - raw);
            }
            self.smoothed_spectrum[i] = self.smoothed_spectrum[i].clamp(0.0, 1.0);

            // Peak cap logic
            let current = self.smoothed_spectrum[i];
            if current >= self.peak_caps[i] {
                self.peak_caps[i] = current;
            } else {
                self.peak_caps[i] = (self.peak_caps[i] - 0.03).max(0.0);
            }
        }

        AudioFrame {
            spectrum: self.smoothed_spectrum.clone(),
            waveform: self.recent_waveform.clone(),
            rms_left: rms_l,
            rms_right: rms_r,
            peak_left: peak_l.min(1.0),
            peak_right: peak_r.min(1.0),
        }
    }
}
