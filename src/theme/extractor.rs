use image::{DynamicImage, GenericImageView};
use ratatui::style::Color;

/// Palette extracted dynamically from album artwork.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtractedPalette {
    pub primary: Color,
    pub secondary: Color,
    pub peak: Color,
}

impl Default for ExtractedPalette {
    fn default() -> Self {
        Self {
            primary: Color::Rgb(244, 63, 94),    // Neon Rose
            secondary: Color::Rgb(56, 189, 248), // Sky Blue
            peak: Color::Rgb(255, 255, 255),      // Electric White
        }
    }
}

/// Extracts dominant accent colors from an image using hue quantization and saturation weighting.
pub fn extract_palette(img: &DynamicImage) -> ExtractedPalette {
    // Downsample image to 48x48 for fast analysis (< 1ms)
    let small = img.resize_exact(48, 48, image::imageops::FilterType::Triangle);
    let (width, height) = small.dimensions();

    // 16 hue bins covering 360 degrees (22.5 deg per bin)
    const NUM_BINS: usize = 16;
    #[derive(Default, Clone)]
    struct Bin {
        count: usize,
        r_sum: u64,
        g_sum: u64,
        b_sum: u64,
        s_sum: f32,
        v_sum: f32,
    }

    let mut bins = vec![Bin::default(); NUM_BINS];
    let mut total_vibrant_pixels = 0;
    let mut gray_r_sum = 0u64;
    let mut gray_g_sum = 0u64;
    let mut gray_b_sum = 0u64;
    let mut gray_count = 0usize;

    for y in 0..height {
        for x in 0..width {
            let pixel = small.get_pixel(x, y);
            let [r, g, b, a] = pixel.0;
            if a < 128 {
                continue;
            }

            let rf = r as f32 / 255.0;
            let gf = g as f32 / 255.0;
            let bf = b as f32 / 255.0;
            let (h, s, v) = rgb_to_hsv(rf, gf, bf);

            // Filter out extreme shadows, blown highlights, and flat grays
            if s >= 0.20 && v >= 0.18 && v <= 0.95 {
                let bin_idx = ((h / 360.0) * NUM_BINS as f32).floor() as usize % NUM_BINS;
                let bin = &mut bins[bin_idx];
                bin.count += 1;
                bin.r_sum += r as u64;
                bin.g_sum += g as u64;
                bin.b_sum += b as u64;
                bin.s_sum += s;
                bin.v_sum += v;
                total_vibrant_pixels += 1;
            } else {
                gray_r_sum += r as u64;
                gray_g_sum += g as u64;
                gray_b_sum += b as u64;
                gray_count += 1;
            }
        }
    }

    // Fallback for monochrome or grayscale artwork
    if total_vibrant_pixels < 20 {
        if gray_count > 0 {
            let avg_r = (gray_r_sum / gray_count as u64).clamp(160, 245) as u8;
            let avg_g = (gray_g_sum / gray_count as u64).clamp(160, 245) as u8;
            let avg_b = (gray_b_sum / gray_count as u64).clamp(170, 255) as u8;
            return ExtractedPalette {
                primary: Color::Rgb(avg_r, avg_g, avg_b),
                secondary: Color::Rgb(120, 140, 165),
                peak: Color::Rgb(255, 255, 255),
            };
        }
        return ExtractedPalette::default();
    }

    // Score bins: prioritize frequency and saturation
    let mut scored: Vec<(usize, f32, Color)> = bins
        .iter()
        .enumerate()
        .filter(|(_, b)| b.count > 0)
        .map(|(idx, b)| {
            let count = b.count as f32;
            let avg_s = b.s_sum / count;
            let avg_v = b.v_sum / count;
            let score = count * (avg_s * 1.5 + 0.3) * (avg_v * 0.8 + 0.2);

            let r = (b.r_sum / b.count as u64).clamp(0, 255) as u8;
            let g = (b.g_sum / b.count as u64).clamp(0, 255) as u8;
            let b_val = (b.b_sum / b.count as u64).clamp(0, 255) as u8;
            (idx, score, Color::Rgb(r, g, b_val))
        })
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    if scored.is_empty() {
        return ExtractedPalette::default();
    }

    let (primary_bin, _, primary) = scored[0];

    // Find secondary accent with distinct hue (at least 2 bins away = ~45 degrees)
    let secondary = scored
        .iter()
        .skip(1)
        .find(|(bin, _, _)| {
            let diff = (*bin as isize - primary_bin as isize).abs();
            let cyclic_diff = diff.min(NUM_BINS as isize - diff);
            cyclic_diff >= 2
        })
        .map(|(_, _, col)| *col)
        .unwrap_or_else(|| {
            // If no distinct second hue, generate a harmonious tint
            match primary {
                Color::Rgb(r, g, b) => Color::Rgb(
                    ((r as u16 + 60).min(255)) as u8,
                    ((g as u16 + 40).min(255)) as u8,
                    b.saturating_sub(30),
                ),
                _ => Color::Rgb(56, 189, 248),
            }
        });

    // Peak color: a lighter, crisp tint of the primary color
    let peak = match primary {
        Color::Rgb(r, g, b) => {
            let pr = (r as f32 * 0.4 + 255.0 * 0.6).round() as u8;
            let pg = (g as f32 * 0.4 + 255.0 * 0.6).round() as u8;
            let pb = (b as f32 * 0.4 + 255.0 * 0.6).round() as u8;
            Color::Rgb(pr, pg, pb)
        }
        _ => Color::Rgb(255, 255, 255),
    };

    ExtractedPalette {
        primary,
        secondary,
        peak,
    }
}

fn rgb_to_hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta).rem_euclid(6.0))
    } else if max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };

    let s = if max == 0.0 { 0.0 } else { delta / max };
    let v = max;

    (h, s, v)
}
