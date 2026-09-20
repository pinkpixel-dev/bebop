use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{OutputCallbackInfo, SampleFormat, Stream, StreamConfig};

/// Shared audio controls between UI thread and CPAL audio callback.
pub struct OutputControls {
    pub volume: Mutex<f32>,
    pub muted: AtomicBool,
    pub paused: AtomicBool,
}

impl OutputControls {
    pub fn new(volume: f32) -> Self {
        Self {
            volume: Mutex::new(volume.clamp(0.0, 1.0)),
            muted: AtomicBool::new(false),
            paused: AtomicBool::new(false),
        }
    }

    pub fn set_volume(&self, vol: f32) {
        if let Ok(mut v) = self.volume.lock() {
            *v = vol.clamp(0.0, 1.0);
        }
    }

    pub fn volume(&self) -> f32 {
        self.volume.lock().map(|v| *v).unwrap_or(0.8)
    }

    pub fn toggle_mute(&self) -> bool {
        let prev = self.muted.load(Ordering::Relaxed);
        self.muted.store(!prev, Ordering::Relaxed);
        !prev
    }

    pub fn is_muted(&self) -> bool {
        self.muted.load(Ordering::Relaxed)
    }

    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Relaxed);
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }
}

#[allow(dead_code)]
pub struct AudioOutput {
    _stream: Stream,
    pub device_sample_rate: u32,
    pub channels: u16,
    pub controls: Arc<OutputControls>,
}

impl AudioOutput {
    /// Initialize CPAL stream with a sample provider callback.
    /// The callback receives `(&mut [f32], volume)` where the callback fills `[L, R, L, R...]`.
    pub fn new<F>(sample_rate: u32, mut sample_callback: F) -> Result<Self, anyhow::Error>
    where
        F: FnMut(&mut [f32]) + Send + 'static,
    {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow::anyhow!("No audio output device found"))?;

        let _supported_configs_range = device.supported_output_configs()?;
        let supported_config = device
            .default_output_config()
            .map_err(|e| anyhow::anyhow!("Failed to get default output config: {}", e))?;

        let channels = supported_config.channels().min(2);
        let stream_config = StreamConfig {
            channels,
            sample_rate: cpal::SampleRate(sample_rate.clamp(22050, 96000)),
            buffer_size: cpal::BufferSize::Default,
        };

        let controls = Arc::new(OutputControls::new(0.8));
        let controls_clone = Arc::clone(&controls);

        let err_fn = |err| {
            eprintln!("Audio output stream error: {}", err);
        };

        let mut scratch_buf = Vec::new();

        let stream = match supported_config.sample_format() {
            SampleFormat::F32 => device.build_output_stream(
                &stream_config,
                move |data: &mut [f32], _: &OutputCallbackInfo| {
                    if controls_clone.is_paused() {
                        data.fill(0.0);
                        return;
                    }

                    let vol = if controls_clone.is_muted() {
                        0.0
                    } else {
                        controls_clone.volume()
                    };

                    sample_callback(data);

                    if vol != 1.0 {
                        for sample in data.iter_mut() {
                            *sample *= vol;
                        }
                    }
                },
                err_fn,
                None,
            )?,
            SampleFormat::I16 => device.build_output_stream(
                &stream_config,
                move |data: &mut [i16], _: &OutputCallbackInfo| {
                    if controls_clone.is_paused() {
                        data.fill(0);
                        return;
                    }

                    let vol = if controls_clone.is_muted() {
                        0.0
                    } else {
                        controls_clone.volume()
                    };

                    scratch_buf.resize(data.len(), 0.0f32);
                    sample_callback(&mut scratch_buf);

                    for (out, &sample) in data.iter_mut().zip(scratch_buf.iter()) {
                        let scaled = (sample * vol * 32767.0).clamp(-32768.0, 32767.0);
                        *out = scaled as i16;
                    }
                },
                err_fn,
                None,
            )?,
            SampleFormat::U16 => device.build_output_stream(
                &stream_config,
                move |data: &mut [u16], _: &OutputCallbackInfo| {
                    if controls_clone.is_paused() {
                        data.fill(32768);
                        return;
                    }

                    let vol = if controls_clone.is_muted() {
                        0.0
                    } else {
                        controls_clone.volume()
                    };

                    scratch_buf.resize(data.len(), 0.0f32);
                    sample_callback(&mut scratch_buf);

                    for (out, &sample) in data.iter_mut().zip(scratch_buf.iter()) {
                        let scaled = (sample * vol * 32767.0).clamp(-32768.0, 32767.0);
                        *out = (scaled + 32768.0) as u16;
                    }
                },
                err_fn,
                None,
            )?,
            _ => return Err(anyhow::anyhow!("Unsupported audio sample format")),
        };

        stream.play()?;

        Ok(Self {
            _stream: stream,
            device_sample_rate: stream_config.sample_rate.0,
            channels: stream_config.channels,
            controls,
        })
    }
}
