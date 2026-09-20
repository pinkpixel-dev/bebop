/// Linear-interpolating sample rate converter for interleaved stereo audio.
///
/// Decoded audio arrives at whatever rate the file was encoded at, while the
/// output stream runs at a fixed device rate. Without this step a 48 kHz track
/// pushed into a 44.1 kHz stream plays back at 0.919x, which sounds slow and
/// about a semitone and a half flat.
pub struct LinearResampler {
    input_rate: u32,
    output_rate: u32,
    /// Input frames consumed per output frame.
    step: f64,
    /// Read position in frames, relative to the start of the buffer being
    /// processed. Negative values point at `last`, the carried-over frame.
    position: f64,
    /// Final frame of the previous buffer, so interpolation stays continuous
    /// across buffer boundaries.
    last: [f32; 2],
}

impl LinearResampler {
    pub fn new(input_rate: u32, output_rate: u32) -> Self {
        let input_rate = input_rate.max(1);
        let output_rate = output_rate.max(1);
        Self {
            input_rate,
            output_rate,
            step: input_rate as f64 / output_rate as f64,
            position: 0.0,
            last: [0.0, 0.0],
        }
    }

    pub fn input_rate(&self) -> u32 {
        self.input_rate
    }

    pub fn output_rate(&self) -> u32 {
        self.output_rate
    }

    /// True when input and output rates match and samples pass through untouched.
    pub fn is_passthrough(&self) -> bool {
        self.input_rate == self.output_rate
    }

    /// Drop carried-over state. Call after a seek so the stale frame from the
    /// old read position does not bleed into the new one.
    pub fn reset(&mut self) {
        self.position = 0.0;
        self.last = [0.0, 0.0];
    }

    /// Resample `input` (interleaved stereo) and append the result to `output`.
    pub fn process(&mut self, input: &[f32], output: &mut Vec<f32>) {
        if input.len() < 2 {
            return;
        }

        if self.is_passthrough() {
            output.extend_from_slice(input);
            return;
        }

        let frames = input.len() / 2;
        let last = self.last;
        let frame_at = |index: isize| -> [f32; 2] {
            if index < 0 {
                last
            } else {
                let i = index as usize * 2;
                [input[i], input[i + 1]]
            }
        };

        // Interpolation needs the frame after `position`, so the last input
        // frame is held back and carried into the next call.
        let limit = (frames - 1) as f64;
        while self.position < limit {
            let base = self.position.floor();
            let frac = (self.position - base) as f32;
            let left = frame_at(base as isize);
            let right = frame_at(base as isize + 1);

            output.push(left[0] + (right[0] - left[0]) * frac);
            output.push(left[1] + (right[1] - left[1]) * frac);

            self.position += self.step;
        }

        self.last = frame_at((frames - 1) as isize);
        self.position -= frames as f64;
    }
}
