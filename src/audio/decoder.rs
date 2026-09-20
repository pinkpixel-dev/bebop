use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::Duration;
use symphonia::core::audio::{AudioBufferRef, SampleBuffer, SignalSpec};
use symphonia::core::codecs::{Decoder, DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;

pub struct AudioDecoder {
    _path: PathBuf,
    format_reader: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    sample_rate: u32,
    channels: usize,
    duration: Duration,
    sample_buf: Option<SampleBuffer<f32>>,
    current_packet_samples: Vec<f32>,
    packet_sample_idx: usize,
    current_time: Duration,
}

impl AudioDecoder {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, anyhow::Error> {
        let path_buf = path.as_ref().to_path_buf();
        let file = File::open(&path_buf)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path_buf.extension().and_then(|s| s.to_str()) {
            hint.with_extension(ext);
        }

        let probed = symphonia::default::get_probe().format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )?;

        let format_reader = probed.format;

        // Find the first playable audio track
        let track = format_reader
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| anyhow::anyhow!("No supported audio track found in file"))?;

        let track_id = track.id;
        let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
        let channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(2);

        // Estimate duration from track time base and n_frames
        let duration = if let (Some(n_frames), Some(tb)) = (track.codec_params.n_frames, track.codec_params.time_base) {
            let time = tb.calc_time(n_frames);
            Duration::from_secs(time.seconds) + Duration::from_secs_f64(time.frac)
        } else {
            Duration::ZERO
        };

        let decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())?;

        Ok(Self {
            _path: path_buf,
            format_reader,
            decoder,
            track_id,
            sample_rate,
            channels,
            duration,
            sample_buf: None,
            current_packet_samples: Vec::new(),
            packet_sample_idx: 0,
            current_time: Duration::ZERO,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> usize {
        self.channels
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }

    pub fn current_time(&self) -> Duration {
        self.current_time
    }

    /// Seek to a specified target time.
    pub fn seek(&mut self, target: Duration) -> Result<(), anyhow::Error> {
        let time = Time::from(target.as_secs_f64());
        self.format_reader.seek(
            SeekMode::Coarse,
            SeekTo::Time {
                time,
                track_id: Some(self.track_id),
            },
        )?;
        self.decoder.reset();
        self.current_packet_samples.clear();
        self.packet_sample_idx = 0;
        self.current_time = target;
        Ok(())
    }

    /// Reads decoded samples into output buffer (interleaved stereo [L, R, L, R...]).
    /// Returns number of stereo sample frames written (channels = 2).
    pub fn read_samples(&mut self, out_buffer: &mut [f32]) -> usize {
        let mut out_idx = 0;
        let needed = out_buffer.len();

        while out_idx < needed {
            // Drain remaining samples from the current packet
            if self.packet_sample_idx < self.current_packet_samples.len() {
                let available = self.current_packet_samples.len() - self.packet_sample_idx;
                let to_copy = available.min(needed - out_idx);

                out_buffer[out_idx..out_idx + to_copy].copy_from_slice(
                    &self.current_packet_samples[self.packet_sample_idx..self.packet_sample_idx + to_copy],
                );

                out_idx += to_copy;
                self.packet_sample_idx += to_copy;
                continue;
            }

            // Need next packet from format reader
            match self.format_reader.next_packet() {
                Ok(packet) => {
                    if packet.track_id() != self.track_id {
                        continue;
                    }

                    match self.decoder.decode(&packet) {
                        Ok(decoded) => {
                            Self::convert_buffer_to_stereo(
                                &mut self.sample_buf,
                                &mut self.current_packet_samples,
                                &decoded,
                            );
                            self.packet_sample_idx = 0;
                        }
                        Err(SymphoniaError::DecodeError(err)) => {
                            // Non-fatal decode errors: skip frame
                            eprintln!("Audio decode warning: {}", err);
                        }
                        Err(err) => {
                            eprintln!("Audio decode error: {}", err);
                            break;
                        }
                    }
                }
                Err(SymphoniaError::IoError(err)) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                    // Reached end of file
                    break;
                }
                Err(_) => {
                    break;
                }
            }
        }

        // Advance current time based on stereo frames read
        let frames_read = out_idx / 2;
        if self.sample_rate > 0 {
            let elapsed_secs = frames_read as f64 / self.sample_rate as f64;
            self.current_time += Duration::from_secs_f64(elapsed_secs);
        }

        out_idx
    }

    /// Converts decoded AudioBufferRef to interleaved stereo f32 samples.
    fn convert_buffer_to_stereo(
        sample_buf: &mut Option<SampleBuffer<f32>>,
        current_packet_samples: &mut Vec<f32>,
        decoded: &AudioBufferRef,
    ) {
        let spec = *decoded.spec();
        let num_frames = decoded.frames();

        if sample_buf.is_none() {
            let signal_spec = SignalSpec::new(spec.rate, spec.channels);
            *sample_buf = Some(SampleBuffer::new(decoded.capacity() as u64, signal_spec));
        }

        if let Some(buf) = sample_buf.as_mut() {
            buf.copy_interleaved_ref(decoded.clone());
            let raw_samples = buf.samples();

            current_packet_samples.clear();
            current_packet_samples.reserve(num_frames * 2);

            let channels = spec.channels.count();
            if channels == 1 {
                // Duplicate mono to stereo [L, R]
                for &s in raw_samples.iter().take(num_frames) {
                    current_packet_samples.push(s);
                    current_packet_samples.push(s);
                }
            } else if channels >= 2 {
                // Keep front left and right
                for chunk in raw_samples.chunks_exact(channels).take(num_frames) {
                    current_packet_samples.push(chunk[0]);
                    current_packet_samples.push(chunk[1]);
                }
            }
        }
    }
}
