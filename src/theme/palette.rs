use ratatui::style::Color;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeMode {
    NeonRainbow,
    VercelDark,
    Ice,
    Sunset,
    Mono,
}

#[derive(Clone, Debug)]
pub struct Theme {
    pub name: &'static str,
    pub is_rainbow: bool,
    pub bg: Color,
    pub surface: Color,
    pub border: Color,
    pub border_focused: Color,
    pub text: Color,
    pub text_muted: Color,
    pub text_dim: Color,
    pub accent: Color,
    pub visualizer_primary: Color,
    pub visualizer_secondary: Color,
    pub visualizer_peak: Color,
    pub progress_elapsed: Color,
    pub progress_remaining: Color,
    pub progress_thumb: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::neon_rainbow()
    }
}

impl Theme {
    /// Vibrant Neon Rainbow theme with multi-stop frequency gradient
    pub fn neon_rainbow() -> Self {
        Self {
            name: "Neon Rainbow",
            is_rainbow: true,
            bg: Color::Rgb(18, 18, 20),
            surface: Color::Rgb(26, 26, 30),
            border: Color::Rgb(48, 48, 54),
            border_focused: Color::Rgb(244, 63, 94), // Neon rose
            text: Color::Rgb(245, 245, 247),
            text_muted: Color::Rgb(150, 150, 160),
            text_dim: Color::Rgb(90, 90, 100),
            accent: Color::Rgb(244, 63, 94),
            visualizer_primary: Color::Rgb(244, 63, 94), // Fallback
            visualizer_secondary: Color::Rgb(6, 182, 212),
            visualizer_peak: Color::Rgb(255, 255, 255), // Bright electric white peak
            progress_elapsed: Color::Rgb(244, 63, 94),
            progress_remaining: Color::Rgb(55, 55, 65),
            progress_thumb: Color::Rgb(255, 255, 255),
        }
    }

    /// Sleek Vercel-inspired dark charcoal theme
    pub fn vercel_dark() -> Self {
        Self {
            name: "Vercel Dark",
            is_rainbow: false,
            bg: Color::Rgb(18, 18, 20),
            surface: Color::Rgb(26, 26, 30),
            border: Color::Rgb(48, 48, 54),
            border_focused: Color::Rgb(180, 180, 190),
            text: Color::Rgb(245, 245, 247),
            text_muted: Color::Rgb(150, 150, 160),
            text_dim: Color::Rgb(90, 90, 100),
            accent: Color::Rgb(255, 255, 255),
            visualizer_primary: Color::Rgb(56, 189, 248), // Sky blue
            visualizer_secondary: Color::Rgb(99, 102, 241), // Indigo
            visualizer_peak: Color::Rgb(244, 114, 182), // Soft pink peak
            progress_elapsed: Color::Rgb(245, 245, 247),
            progress_remaining: Color::Rgb(55, 55, 65),
            progress_thumb: Color::Rgb(255, 255, 255),
        }
    }

    pub fn ice() -> Self {
        Self {
            name: "Ice",
            is_rainbow: false,
            bg: Color::Rgb(15, 20, 25),
            surface: Color::Rgb(22, 30, 38),
            border: Color::Rgb(45, 60, 75),
            border_focused: Color::Rgb(140, 200, 240),
            text: Color::Rgb(240, 248, 255),
            text_muted: Color::Rgb(140, 170, 195),
            text_dim: Color::Rgb(80, 105, 125),
            accent: Color::Rgb(125, 211, 252),
            visualizer_primary: Color::Rgb(56, 189, 248),
            visualizer_secondary: Color::Rgb(14, 165, 233),
            visualizer_peak: Color::Rgb(186, 230, 253),
            progress_elapsed: Color::Rgb(125, 211, 252),
            progress_remaining: Color::Rgb(35, 50, 65),
            progress_thumb: Color::Rgb(240, 248, 255),
        }
    }

    pub fn mono() -> Self {
        Self {
            name: "Mono",
            is_rainbow: false,
            bg: Color::Rgb(16, 16, 18),
            surface: Color::Rgb(24, 24, 28),
            border: Color::Rgb(50, 50, 55),
            border_focused: Color::Rgb(220, 220, 225),
            text: Color::Rgb(250, 250, 250),
            text_muted: Color::Rgb(160, 160, 165),
            text_dim: Color::Rgb(95, 95, 100),
            accent: Color::Rgb(255, 255, 255),
            visualizer_primary: Color::Rgb(230, 230, 235),
            visualizer_secondary: Color::Rgb(160, 160, 165),
            visualizer_peak: Color::Rgb(255, 255, 255),
            progress_elapsed: Color::Rgb(250, 250, 250),
            progress_remaining: Color::Rgb(55, 55, 60),
            progress_thumb: Color::Rgb(255, 255, 255),
        }
    }

    pub fn sunset() -> Self {
        Self {
            name: "Sunset",
            is_rainbow: false,
            bg: Color::Rgb(20, 16, 18),
            surface: Color::Rgb(30, 24, 28),
            border: Color::Rgb(65, 45, 55),
            border_focused: Color::Rgb(251, 146, 60),
            text: Color::Rgb(255, 245, 245),
            text_muted: Color::Rgb(180, 145, 155),
            text_dim: Color::Rgb(110, 85, 95),
            accent: Color::Rgb(251, 146, 60),
            visualizer_primary: Color::Rgb(251, 113, 133), // Rose
            visualizer_secondary: Color::Rgb(249, 115, 22), // Orange
            visualizer_peak: Color::Rgb(253, 224, 71), // Amber yellow
            progress_elapsed: Color::Rgb(251, 146, 60),
            progress_remaining: Color::Rgb(60, 45, 52),
            progress_thumb: Color::Rgb(255, 255, 255),
        }
    }

    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "vercel dark" | "vercel" => Self::vercel_dark(),
            "ice" => Self::ice(),
            "sunset" => Self::sunset(),
            "mono" => Self::mono(),
            _ => Self::neon_rainbow(),
        }
    }

    pub fn cycle_next(&self) -> Self {
        match self.name {
            "Neon Rainbow" => Self::vercel_dark(),
            "Vercel Dark" => Self::ice(),
            "Ice" => Self::sunset(),
            "Sunset" => Self::mono(),
            _ => Self::neon_rainbow(),
        }
    }

    /// Computes the visualizer color given horizontal position (0.0 to 1.0) and vertical height (0.0 to 1.0).
    pub fn get_bar_color(&self, freq_t: f32, height_t: f32) -> Color {
        if self.is_rainbow {
            // Multi-stop Neon Rainbow spectrum across frequencies
            rainbow_gradient(freq_t, height_t)
        } else {
            interpolate_color(self.visualizer_primary, self.visualizer_secondary, height_t)
        }
    }
}

/// Computes multi-stop neon rainbow gradient:
/// Bass (hot pink/rose) -> Violet -> Electric Blue -> Cyan -> Emerald Green -> Amber/Gold (Treble)
fn rainbow_gradient(freq_t: f32, height_t: f32) -> Color {
    let stops = [
        (0.00, Color::Rgb(255, 42, 133)),  // Neon Magenta / Pink
        (0.20, Color::Rgb(168, 85, 247)),  // Violet
        (0.40, Color::Rgb(59, 130, 246)),  // Electric Blue
        (0.60, Color::Rgb(6, 182, 212)),   // Neon Cyan
        (0.80, Color::Rgb(16, 185, 129)),  // Spring Green
        (1.00, Color::Rgb(245, 158, 11)),  // Amber / Bright Gold
    ];

    let t = freq_t.clamp(0.0, 1.0);
    let mut base_color = stops[0].1;

    for i in 0..stops.len() - 1 {
        let (p1, c1) = stops[i];
        let (p2, c2) = stops[i + 1];
        if t >= p1 && t <= p2 {
            let local_t = (t - p1) / (p2 - p1);
            base_color = interpolate_color(c1, c2, local_t);
            break;
        }
    }

    // Slightly brighten upper rows for dimension without blur
    if height_t > 0.6 {
        let boost = ((height_t - 0.6) * 0.5).min(0.25);
        interpolate_color(base_color, Color::Rgb(255, 255, 255), boost)
    } else {
        base_color
    }
}

pub fn interpolate_color(c1: Color, c2: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    match (c1, c2) {
        (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
            let r = (r1 as f32 * (1.0 - t) + r2 as f32 * t).round() as u8;
            let g = (g1 as f32 * (1.0 - t) + g2 as f32 * t).round() as u8;
            let b = (b1 as f32 * (1.0 - t) + b2 as f32 * t).round() as u8;
            Color::Rgb(r, g, b)
        }
        _ => c1,
    }
}
