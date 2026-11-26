// Dithered temple backdrop behind the empty chat landing, ported from the web
// client's EmptyChatBackdrop.tsx (upstream commit bfb3bd3551). The pipeline is
// the same: the temple scene is resized to cover the pane at one canvas pixel
// per dot cell, blurred, unsharp-masked (light), tone-mapped through the same
// smoothstep levels, ordered-dithered with the classic 8x8 Bayer matrix and
// de-strayed; dots print in the theme accent (softened toward white in light
// mode). Ripples and the animated gradient style are not ported yet.

use gpui_kit::*;
use std::sync::OnceLock;

const CELL_PX: f32 = 2.0;
const SCENE_ASPECT: f32 = 9.0 / 16.0;
const SCENE_MIN_WIDTH_PX: f32 = 1600.0;
const SHARPEN_RADIUS_CELLS: f32 = 4.0;
const LIGHT_INK_SOFTEN: f32 = 0.28;
const GROUND_DIM: (f32, f32, f32, f32, f32) = (0.55, 0.6, 0.16, 0.07, 0.18);
const STRAY_RADIUS_CELLS: i32 = 2;

struct Levels {
    low: f32,
    high: f32,
    max: f32,
    fade_from: f32,
    fade: f32,
    thin_from: f32,
    thin_to: f32,
    dot: f32,
    invert: bool,
    blur: f32,
    sharpen: f32,
    sharpen_from: f32,
    sharpen_to: f32,
    thin_right: f32,
    quiet_floor: f32,
}

const DARK_LEVELS: Levels = Levels {
    low: 0.14,
    high: 0.9,
    max: 0.85,
    fade_from: 0.0,
    fade: 0.12,
    thin_from: 1.0,
    thin_to: 1.0,
    dot: 0.8,
    invert: false,
    blur: 0.6,
    sharpen: 0.0,
    sharpen_from: 0.0,
    sharpen_to: 0.0,
    thin_right: 0.0,
    quiet_floor: 0.6,
};

const LIGHT_LEVELS: Levels = Levels {
    low: 0.6,
    high: 0.8,
    max: 1.0,
    fade_from: 0.0,
    fade: 0.12,
    thin_from: 0.7,
    thin_to: 0.7,
    dot: 1.0,
    invert: true,
    blur: 0.3,
    sharpen: 1.6,
    sharpen_from: 0.24,
    sharpen_to: 0.36,
    thin_right: 0.45,
    quiet_floor: 0.6,
};

const BAYER_8: [u8; 64] = {
    let vals = [
        0u32, 32, 8, 40, 2, 34, 10, 42, 48, 16, 56, 24, 50, 18, 58, 26, 12, 44, 4, 36, 14, 46, 6,
        38, 60, 28, 52, 20, 62, 30, 54, 22, 3, 35, 11, 43, 1, 33, 9, 41, 51, 19, 59, 27, 49, 17,
        57, 25, 15, 47, 7, 39, 13, 45, 5, 37, 63, 31, 55, 23, 61, 29, 53, 21,
    ];
    let mut out = [0u8; 64];
    let mut i = 0;
    while i < 64 {
        out[i] = (((vals[i] as f32 + 0.5) / 64.0) * 255.0).round() as u8;
        i += 1;
    }
    out
};

fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn scene() -> &'static image::RgbaImage {
    static SCENE: OnceLock<image::RgbaImage> = OnceLock::new();
    SCENE.get_or_init(|| {
        let bytes = include_bytes!("../assets/empty-chat-scene.jpg");
        image::load_from_memory_with_format(bytes, image::ImageFormat::Jpeg)
            .expect("scene jpeg decodes")
            .to_rgba8()
    })
}

/// Bilinear resize of the scene into the cell-resolution grid area, matching
/// the canvas drawImage(dst) upstream: the scene covers [scene_w x scene_h]
/// cells placed at (left, top) in cell units; out-of-scene cells stay empty
/// (alpha 0) exactly like the cleared canvas.
fn draw_scene_cells(
    cols: usize,
    rows: usize,
    left: f32,
    top: f32,
    scene_w: f32,
    scene_h: f32,
) -> Vec<f32> {
    // returns luminance-per-cell with alpha folded in: -1 marks empty cells
    let scene = scene();
    let (sw, sh) = (scene.width() as f32, scene.height() as f32);
    let mut lum = vec![-1.0f32; cols * rows];
    // cell (col,row) samples the scene at its center in scene-cell space
    for row in 0..rows {
        for col in 0..cols {
            let cx = col as f32 + 0.5;
            let cy = row as f32 + 0.5;
            // position within the drawn scene rect (cell units)
            let sx = (cx - left) / scene_w * sw;
            let sy = (cy - top) / scene_h * sh;
            if sx < 0.0 || sy < 0.0 || sx >= sw - 1.0 || sy >= sh - 1.0 {
                continue;
            }
            let x0 = sx.floor() as u32;
            let y0 = sy.floor() as u32;
            let fx = sx - x0 as f32;
            let fy = sy - y0 as f32;
            let sample = |x: u32, y: u32| -> f32 {
                let p = scene.get_pixel(x, y).0;
                (0.2126 * p[0] as f32 + 0.7152 * p[1] as f32 + 0.0722 * p[2] as f32) / 255.0
            };
            let a = sample(x0, y0);
            let b = sample(x0 + 1, y0);
            let c = sample(x0, y0 + 1);
            let d = sample(x0 + 1, y0 + 1);
            let topmix = a + (b - a) * fx;
            let botmix = c + (d - c) * fx;
            lum[row * cols + col] = topmix + (botmix - topmix) * fy;
        }
    }
    lum
}

/// Separable Gaussian-ish blur over the luminance grid (empty cells act as
/// transparent: they contribute nothing and renormalize the kernel, matching
/// canvas edge behavior closely enough for a blur this small).
fn blur_lum(src: &[f32], cols: usize, rows: usize, radius: f32) -> Vec<f32> {
    if radius <= 0.01 {
        return src.to_vec();
    }
    // canvas blur(N) is a Gaussian with sigma = N (measured against Chrome)
    let sigma = radius.max(0.2);
    let spread = (sigma * 3.0).ceil() as i32;
    let kernel: Vec<f32> = (-spread..=spread)
        .map(|i| (-0.5 * (i as f32 / sigma).powi(2)).exp())
        .collect();
    let pass = |input: &[f32], horizontal: bool| -> Vec<f32> {
        let mut out = vec![-1.0f32; cols * rows];
        for row in 0..rows {
            for col in 0..cols {
                let mut acc = 0.0f32;
                let mut wsum = 0.0f32;
                for (ki, kv) in kernel.iter().enumerate() {
                    let off = ki as i32 - spread;
                    let (x, y) = if horizontal {
                        (col as i32 + off, row as i32)
                    } else {
                        (col as i32, row as i32 + off)
                    };
                    if x < 0 || y < 0 || x >= cols as i32 || y >= rows as i32 {
                        continue;
                    }
                    let v = input[y as usize * cols + x as usize];
                    if v < 0.0 {
                        continue;
                    }
                    acc += v * kv;
                    wsum += kv;
                }
                if src[row * cols + col] >= 0.0 && wsum > 0.0 {
                    out[row * cols + col] = acc / wsum;
                }
            }
        }
        out
    };
    let tmp = pass(src, true);
    pass(&tmp, false)
}

/// Rects in px the landing UI occupies: dots dim behind them (upstream measures
/// the DOM; the shell passes its own layout zones).
pub struct BackdropSpec {
    pub width: f32,
    pub height: f32,
    pub dark: bool,
    pub accent_rgb: [u8; 3],
    /// (x, y, w, h) quiet zones in logical px, container coords.
    pub quiet: Vec<(f32, f32, f32, f32)>,
}

const QUIET_PADDING_PX: f32 = 12.0;
const QUIET_FEATHER_PX: f32 = 72.0;

fn quiet_factor(x: f32, y: f32, zones: &[(f32, f32, f32, f32)], floor: f32) -> f32 {
    let mut factor = 1.0f32;
    for &(zx, zy, zw, zh) in zones {
        let dx = (zx - QUIET_PADDING_PX - x)
            .max(0.0)
            .max(x - (zx + zw) - QUIET_PADDING_PX);
        let dy = (zy - QUIET_PADDING_PX - y)
            .max(0.0)
            .max(y - (zy + zh) - QUIET_PADDING_PX);
        factor = factor.min(smoothstep(0.0, QUIET_FEATHER_PX, (dx * dx + dy * dy).sqrt()));
    }
    floor + (1.0 - floor) * factor
}

/// Builds the dot field and renders it into a BGRA bitmap at logical
/// resolution (each cell is 2x2 px; the dot circle is anti-aliased by
/// coverage). Returns None when nothing would show.
pub fn build_backdrop_image(spec: &BackdropSpec) -> Option<std::sync::Arc<RenderImage>> {
    let width = spec.width;
    let height = spec.height;
    if width < 8.0 || height < 8.0 {
        return None;
    }
    let cols = (width / CELL_PX).ceil() as usize;
    let rows = (height / CELL_PX).ceil() as usize;
    let levels = if spec.dark { &DARK_LEVELS } else { &LIGHT_LEVELS };

    let scene_w = width.max(SCENE_MIN_WIDTH_PX).max(height / SCENE_ASPECT);
    let scene_h = scene_w * SCENE_ASPECT;
    let left = (width - scene_w) / 2.0;
    let top = height - scene_h;
    // to cell units
    let cleft = left / CELL_PX;
    let ctop = top / CELL_PX;
    let cscene_w = scene_w / CELL_PX;
    let cscene_h = scene_h / CELL_PX;

    let base = draw_scene_cells(cols, rows, cleft, ctop, cscene_w, cscene_h);
    let blurred = blur_lum(&base, cols, rows, levels.blur);
    let wide = if levels.sharpen > 0.0 {
        Some(blur_lum(&base, cols, rows, SHARPEN_RADIUS_CELLS))
    } else {
        None
    };

    // density per cell
    let mut density = vec![0u8; cols * rows];
    let fade_start = top + scene_h * levels.fade_from;
    let fade_end = top + scene_h * levels.fade;
    for row in 0..rows {
        let y = row as f32 * CELL_PX + CELL_PX / 2.0;
        let thin = 1.0 - (1.0 - levels.thin_to) * smoothstep(height * levels.thin_from, height, y);
        let row_bottom = smoothstep(height * 0.45, height * 0.8, y);
        let sharpen = levels.sharpen
            * smoothstep(
                top + scene_h * levels.sharpen_from,
                top + scene_h * levels.sharpen_to,
                y,
            );
        let fade = smoothstep(fade_start, fade_end, y) * thin;
        if fade == 0.0 {
            continue;
        }
        for col in 0..cols {
            let index = row * cols + col;
            let mut luminance = blurred[index];
            if luminance < 0.0 {
                continue;
            }
            if let Some(w) = &wide {
                let around = w[index];
                if around >= 0.0 {
                    luminance += sharpen * (luminance - around);
                }
            }
            let tone = smoothstep(
                levels.low,
                levels.high,
                if levels.invert {
                    1.0 - luminance
                } else {
                    luminance
                },
            );
            if tone == 0.0 {
                continue;
            }
            let right = if levels.thin_right > 0.0 {
                1.0 - levels.thin_right
                    * row_bottom
                    * smoothstep(width * 0.35, width * 0.85, col as f32 * CELL_PX)
            } else {
                1.0
            };
            let gx = (col as f32 * CELL_PX - width * GROUND_DIM.0) / (width * GROUND_DIM.2);
            let gy = (y - height * GROUND_DIM.1) / (height * GROUND_DIM.3);
            let ground = 1.0 - GROUND_DIM.4 * (-(gx * gx + gy * gy)).exp();
            let quiet = if spec.quiet.is_empty() {
                1.0
            } else {
                quiet_factor(col as f32 * CELL_PX, y, &spec.quiet, levels.quiet_floor)
            };
            density[index] =
                (255.0 * levels.max * tone * fade * quiet * right * ground).round() as u8;
        }
    }

    // ordered dither + stray cleanup
    let mut dots = vec![0u8; cols * rows];
    for row in 0..rows {
        for col in 0..cols {
            let index = row * cols + col;
            // BAYER_8 is already normalized to 0..255 thresholds the way
            // upstream does (Math.round(((v + 0.5) / 64) * 255)); the compare
            // is density > threshold, one binary cell pixel, no radius.
            if density[index] > BAYER_8[(row & 7) * 8 + (col & 7)] {
                dots[index] = 1;
            }
        }
    }
    let mut clean = dots.clone();
    for row in 0..rows {
        for col in 0..cols {
            let index = row * cols + col;
            if dots[index] == 0 {
                continue;
            }
            let mut lonely = true;
            'outer: for dy in -STRAY_RADIUS_CELLS..=STRAY_RADIUS_CELLS {
                let yy = row as i32 + dy;
                if yy < 0 || yy >= rows as i32 {
                    continue;
                }
                for dx in -STRAY_RADIUS_CELLS..=STRAY_RADIUS_CELLS {
                    let xx = col as i32 + dx;
                    if (dx != 0 || dy != 0) && xx >= 0 && xx < cols as i32 {
                        if dots[yy as usize * cols + xx as usize] == 1 {
                            lonely = false;
                            break 'outer;
                        }
                    }
                }
            }
            if lonely {
                clean[index] = 0;
            }
        }
    }

    // ink color: light mode softens the accent toward white
    let ink = if !spec.dark {
        spec.accent_rgb
            .map(|c| (c as f32 + (255.0 - c as f32) * LIGHT_INK_SOFTEN).round() as u8)
    } else {
        spec.accent_rgb
    };

    // render one bitmap pixel per cell, binary — upstream's canvas is cols x
    // rows and the browser scales it up with smoothing; gpui does the same
    // when this image is drawn at pane size, which is what softens the dots
    // into the pale print instead of hard full-ink circles.
    let w_px = cols;
    let h_px = rows;
    let mut buf = vec![0u8; w_px * h_px * 4];
    // light sky mask: dot opacity fades in over the top 42% of the pane
    // (upstream LIGHT_SKY_MASK intersecting the dot mask).
    for row in 0..rows {
        for col in 0..cols {
            if clean[row * cols + col] == 0 {
                continue;
            }
            let cy = row as f32 * CELL_PX + CELL_PX / 2.0;
            let sky_alpha = if !spec.dark {
                (cy / (height * 0.42)).clamp(0.0, 1.0)
            } else {
                1.0
            };
            if sky_alpha <= 0.0 {
                continue;
            }
            let idx = (row * w_px + col) * 4;
            let a = (255.0 * sky_alpha).round() as u8;
            // BGRA, premultiplied: gpui composites src.rgb + dst*(1-a),
            // so unpremultiplied ink would print full-strength even
            // where the sky mask keeps alpha low.
            let scale = a as f32 / 255.0;
            buf[idx] = (ink[2] as f32 * scale).round() as u8;
            buf[idx + 1] = (ink[1] as f32 * scale).round() as u8;
            buf[idx + 2] = (ink[0] as f32 * scale).round() as u8;
            buf[idx + 3] = a;
        }
    }

    let img = image::RgbaImage::from_raw(w_px as u32, h_px as u32, buf)?;
    Some(std::sync::Arc::new(RenderImage::new(smallvec::smallvec![
        image::Frame::new(img)
    ])))
}
