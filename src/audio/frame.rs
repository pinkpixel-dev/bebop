/// Audio frame data consumed by visualizers during UI rendering.
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct AudioFrame {
    /// Normalized frequency bin magnitudes between 0.0 and 1.0.
    pub spectrum: Vec<f32>,
    /// Recent time-domain audio samples between -1.0 and 1.0.
    pub waveform: Vec<f32>,
    /// RMS energy of left channel (0.0 to 1.0).
    pub rms_left: f32,
    /// RMS energy of right channel (0.0 to 1.0).
    pub rms_right: f32,
    /// Peak amplitude of left channel (0.0 to 1.0).
    pub peak_left: f32,
    /// Peak amplitude of right channel (0.0 to 1.0).
    pub peak_right: f32,
}

impl Default for AudioFrame {
    fn default() -> Self {
        Self {
            spectrum: vec![0.0; 64],
            waveform: vec![0.0; 256],
            rms_left: 0.0,
            rms_right: 0.0,
            peak_left: 0.0,
            peak_right: 0.0,
        }
    }
}
