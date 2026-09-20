use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::Frame;

use crate::audio::frame::AudioFrame;
use crate::theme::Theme;
use crate::visualizers::Visualizer;

const PARTICLE_SYMBOLS: [char; 5] = ['·', '•', '*', '✧', '✦'];

/// How hard a bass transient throws a particle upward. Divided by particle
/// weight, so light particles fly further on the same kick.
const UPDRAFT: f32 = 2.8;
/// Steady lift from overall energy, which is what raises the whole field
/// during a loud passage and lets it sink during a quiet one.
const SUSTAIN_LIFT: f32 = 0.014;
/// Downward pull per frame. Constant rather than weight-scaled: weight already
/// divides the lift, and scaling both ends made hover height vary with the
/// square of weight, which pinned light particles to the ceiling and left
/// heavy ones stuck on the floor.
const GRAVITY: f32 = 0.035;
/// Air thins toward the top. Upward motion is damped by up to this fraction at
/// the ceiling and not at all at the floor, which is what gives the field a
/// stable hover height instead of an all-or-nothing drift to one edge.
const ALTITUDE_DRAG: f32 = 0.70;
/// Per-frame velocity retention. Lower values make the field settle faster.
const VERTICAL_DRAG: f32 = 0.90;
const HORIZONTAL_DRAG: f32 = 0.90;
/// Velocity clamps in cells per frame, so a loud passage cannot launch a
/// particle off the panel in a single tick.
const MAX_RISE: f32 = 1.6;
const MAX_FALL: f32 = 1.2;
/// Sideways sway driven by treble content.
const SWAY: f32 = 0.10;

#[derive(Clone, Debug)]
struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    energy: f32,
    weight: f32,
    seed: u32,
}

/// Ambient updraft audio particle field visualizer.
/// Floating particles react to bass updrafts and treble sparkles.
pub struct ParticlesVisualizer {
    particles: Vec<Particle>,
    last_width: usize,
    last_height: usize,
    rng_state: u32,
    /// Slow-moving average of bass level. Updrafts come from bass *above* this
    /// baseline, so a steadily loud mix reads as calm rather than as one long
    /// impulse that holds every particle against the ceiling.
    bass_baseline: f32,
    /// Frame counter driving the sway oscillation.
    frame: u32,
}

impl Default for ParticlesVisualizer {
    fn default() -> Self {
        Self {
            particles: Vec::new(),
            last_width: 0,
            last_height: 0,
            rng_state: 123456789,
            bass_baseline: 0.0,
            frame: 0,
        }
    }
}

impl ParticlesVisualizer {
    fn rand_f32(&mut self) -> f32 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 17;
        self.rng_state ^= self.rng_state << 5;
        (self.rng_state as f32) / (u32::MAX as f32)
    }

    fn init_particles(&mut self, width: usize, height: usize) {
        self.particles.clear();
        let count = ((width * height) / 16).clamp(24, 120);

        for i in 0..count {
            let x = self.rand_f32() * width as f32;
            let y = (height as f32 * 0.3) + self.rand_f32() * (height as f32 * 0.65);
            let weight = 0.7 + self.rand_f32() * 0.6;
            self.particles.push(Particle {
                x,
                y,
                vx: 0.0,
                vy: 0.0,
                energy: 0.0,
                weight,
                seed: (i as u32).wrapping_mul(987654321).wrapping_add(1),
            });
        }
        self.last_width = width;
        self.last_height = height;
    }
}

impl Visualizer for ParticlesVisualizer {
    fn name(&self) -> &'static str {
        "Particle Field"
    }

    fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        audio: &AudioFrame,
        theme: &Theme,
    ) {
        if area.width < 4 || area.height < 2 {
            return;
        }

        let width = area.width as usize;
        let height = area.height as usize;
        let buf = frame.buffer_mut();

        if width != self.last_width || height != self.last_height || self.particles.is_empty() {
            self.init_particles(width, height);
        }

        // Calculate frequency bands
        let spec = &audio.spectrum;
        let spec_len = spec.len();
        let (bass, mid, treble) = if spec_len >= 8 {
            let bass_end = (spec_len * 15 / 100).max(2);
            let mid_end = (spec_len * 50 / 100).max(bass_end + 1);

            let bass_avg: f32 = spec[0..bass_end].iter().sum::<f32>() / bass_end as f32;
            let mid_avg: f32 = spec[bass_end..mid_end].iter().sum::<f32>() / (mid_end - bass_end) as f32;
            let treble_avg: f32 = spec[mid_end..].iter().sum::<f32>() / (spec_len - mid_end) as f32;

            (bass_avg.min(1.0), mid_avg.min(1.0), treble_avg.min(1.0))
        } else {
            (0.0, 0.0, 0.0)
        };

        let overall_energy = bass * 0.55 + mid * 0.30 + treble * 0.15;

        // Track the baseline and treat only the excess as a kick. This is what
        // keeps the field bouncing instead of pinned: sustained bass raises the
        // baseline within a second or so and stops producing lift.
        self.bass_baseline = self.bass_baseline * 0.94 + bass * 0.06;
        let kick = (bass - self.bass_baseline).max(0.0);

        self.frame = self.frame.wrapping_add(1);
        let sway_phase = self.frame as f32 * 0.18;

        // Update physics and render each particle
        for i in 0..self.particles.len() {
            let p = &mut self.particles[i];

            // Bass transient throws the particle upward against gravity
            if kick > 0.01 {
                p.vy -= (kick * UPDRAFT) / p.weight;
            }

            // Treble sway, phase-offset per particle so the field does not
            // drift sideways as one block
            let phase = sway_phase + (p.seed % 628) as f32 * 0.01;
            p.vx += phase.sin() * treble * SWAY;

            // Steady lift from the current energy level
            p.vy -= (overall_energy * SUSTAIN_LIFT) / p.weight;

            // Air thins with altitude, so rising costs more the higher a
            // particle already is
            if p.vy < 0.0 {
                let altitude = 1.0 - (p.y / height as f32);
                p.vy *= 1.0 - ALTITUDE_DRAG * altitude;
            }

            // Air drag & gravity
            p.vx *= HORIZONTAL_DRAG;
            p.vy = (p.vy * VERTICAL_DRAG) + GRAVITY;
            p.vy = p.vy.clamp(-MAX_RISE, MAX_FALL);

            p.x += p.vx;
            p.y += p.vy;

            // Boundaries: bounce off both the floor and the ceiling so a
            // particle that reaches an edge comes back rather than sticking
            if p.y >= height as f32 - 1.0 {
                p.y = height as f32 - 1.0;
                p.vy = -p.vy * 0.25;
            } else if p.y < 0.0 {
                p.y = 0.0;
                p.vy = -p.vy * 0.30;
            }

            if p.x < 0.0 {
                p.x = width as f32 - 1.0;
            } else if p.x >= width as f32 {
                p.x = 0.0;
            }

            // Particle energy tracking
            p.energy = (p.energy * 0.85) + (overall_energy * 0.15);

            // Screen coordinates
            let screen_x = area.left() + (p.x.round() as u16).min(area.width.saturating_sub(1));
            let screen_y = area.top() + (p.y.round() as u16).min(area.height.saturating_sub(1));

            if let Some(cell) = buf.cell_mut((screen_x, screen_y)) {
                let (ch, style) = if p.energy >= 0.80 {
                    (
                        PARTICLE_SYMBOLS[4], // '✦'
                        Style::default().fg(theme.visualizer_peak).add_modifier(Modifier::BOLD),
                    )
                } else if p.energy >= 0.62 {
                    (
                        PARTICLE_SYMBOLS[3], // '✧'
                        Style::default().fg(theme.visualizer_primary),
                    )
                } else if p.energy >= 0.42 {
                    (
                        PARTICLE_SYMBOLS[2], // '*'
                        Style::default().fg(theme.visualizer_secondary),
                    )
                } else if p.energy >= 0.22 {
                    (
                        PARTICLE_SYMBOLS[1], // '•'
                        Style::default().fg(theme.text_muted),
                    )
                } else {
                    (
                        PARTICLE_SYMBOLS[0], // '·'
                        Style::default().fg(theme.text_dim),
                    )
                };

                cell.set_char(ch);
                cell.set_style(style);
            }
        }
    }
}
