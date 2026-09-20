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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioDeviceInfo {
    pub name: String,
    pub is_default: bool,
    pub is_active: bool,
}

/// Enumerate all available audio output devices from the host.
pub fn list_output_devices(active_device: Option<&str>) -> Vec<AudioDeviceInfo> {
    let host = cpal::default_host();
    let default_name = host.default_output_device().and_then(|d| d.name().ok());

    let mut devices = Vec::new();
    if let Ok(dev_iter) = host.output_devices() {
        for dev in dev_iter {
            if let Ok(name) = dev.name() {
                let is_default = default_name.as_deref() == Some(&name);
                let is_active = match active_device {
                    Some(act) => act == name,
                    None => is_default,
                };
                devices.push(AudioDeviceInfo {
                    name,
                    is_default,
                    is_active,
                });
            }
        }
    }
    devices
}

/// Rate most current hardware and sound servers run at. Picking it avoids a
/// conversion for the majority of files, which are 48 kHz.
const PREFERRED_SAMPLE_RATE: u32 = 48000;

/// Pick the rate to open the stream at.
///
/// Plug devices such as ALSA's `default` report an arbitrary rate as their
/// default while accepting almost anything, so the reported value is not a
/// reliable signal on its own. Prefer 48 kHz whenever the device supports it
/// and fall back to whatever the device named.
fn choose_sample_rate(device: &cpal::Device, default_rate: u32) -> u32 {
    if default_rate == PREFERRED_SAMPLE_RATE {
        return PREFERRED_SAMPLE_RATE;
    }

    let supported = device
        .supported_output_configs()
        .map(|mut configs| {
            configs.any(|c| {
                c.min_sample_rate().0 <= PREFERRED_SAMPLE_RATE
                    && c.max_sample_rate().0 >= PREFERRED_SAMPLE_RATE
            })
        })
        .unwrap_or(false);

    if supported {
        PREFERRED_SAMPLE_RATE
    } else {
        default_rate
    }
}

#[allow(dead_code)]
pub struct AudioOutput {
    _stream: Stream,
    pub device_sample_rate: u32,
    pub channels: u16,
    pub controls: Arc<OutputControls>,
    pub device_name: String,
}

impl AudioOutput {
    /// Initialize CPAL stream with a sample provider callback.
    /// The callback receives `(&mut [f32], volume)` where the callback fills `[L, R, L, R...]`.
    ///
    /// The stream runs at the device's own default sample rate. Callers are
    /// responsible for delivering samples at `device_sample_rate`.
    pub fn new<F>(
        preferred_device: Option<&str>,
        mut sample_callback: F,
    ) -> Result<Self, anyhow::Error>
    where
        F: FnMut(&mut [f32]) + Send + 'static,
    {
        let host = cpal::default_host();
        let (device, dev_name) = if let Some(target) = preferred_device {
            if let Ok(mut devs) = host.output_devices() {
                if let Some(d) = devs.find(|d| d.name().map(|n| n == target).unwrap_or(false)) {
                    let n = d.name().unwrap_or_else(|_| target.to_string());
                    (d, n)
                } else {
                    let d = host
                        .default_output_device()
                        .ok_or_else(|| anyhow::anyhow!("No audio output device found"))?;
                    let n = d.name().unwrap_or_else(|_| "Default".to_string());
                    (d, n)
                }
            } else {
                let d = host
                    .default_output_device()
                    .ok_or_else(|| anyhow::anyhow!("No audio output device found"))?;
                let n = d.name().unwrap_or_else(|_| "Default".to_string());
                (d, n)
            }
        } else {
            let d = host
                .default_output_device()
                .ok_or_else(|| anyhow::anyhow!("No audio output device found"))?;
            let n = d.name().unwrap_or_else(|_| "Default".to_string());
            (d, n)
        };

        let supported_config = device
            .default_output_config()
            .map_err(|e| anyhow::anyhow!("Failed to get default output config: {}", e))?;

        let channels = supported_config.channels().min(2);
        let sample_rate = choose_sample_rate(&device, supported_config.sample_rate().0);
        let stream_config = StreamConfig {
            channels,
            sample_rate: cpal::SampleRate(sample_rate),
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
            device_name: dev_name,
        })
    }
}
