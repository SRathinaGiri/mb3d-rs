//! Finishing of the picture (not in MB3D): exposure, glow around bright
//! light, filmic tone mapping, contrast, saturation, warmth, a vignette and
//! the colour fringes of a lens, as a last pass over the whole picture
//! (after the depth of field), in floating point so that light above white
//! is kept until the tone mapping.

/// The finishing settings (`Scene::finish`); the defaults change nothing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Finish {
    /// exposure in stops (+1 twice as bright)
    pub exposure: f32,
    /// filmic tone mapping: highlights roll off softly instead of clipping
    pub tonemap: bool,
    /// the glow's strength (0 = none)
    pub glow: f32,
    /// the glow's reach, a part of the picture's diagonal
    pub glow_radius: f32,
    /// the brightness from which light glows (0..1 of white; above 1 only
    /// light above white)
    pub glow_threshold: f32,
    /// -1..1: flatter .. more contrast
    pub contrast: f32,
    /// 0 grey, 1 as it is, 2 twice as colourful
    pub saturation: f32,
    /// -1 cooler (blue) .. 1 warmer (orange)
    pub warmth: f32,
    /// 0..1: darker corners
    pub vignette: f32,
    /// 0..1: red and blue fringes towards the corners, as a lens shows
    pub chromatic: f32,
}

impl Default for Finish {
    fn default() -> Self {
        Finish { exposure: 0.0, tonemap: false, glow: 0.0, glow_radius: 0.02, glow_threshold: 0.8, contrast: 0.0, saturation: 1.0, warmth: 0.0, vignette: 0.0, chromatic: 0.0 }
    }
}

impl Finish {
    /// Changes nothing (the picture stays as painted, byte for byte).
    pub fn is_neutral(&self) -> bool {
        let d = Finish { glow_radius: self.glow_radius, glow_threshold: self.glow_threshold, ..Default::default() };
        *self == d
    }

    /// The scene keys of these settings (`Scene::extra_text`).
    pub fn to_text(&self) -> String {
        format!(
            "finish_exposure = {}\nfinish_tonemap = {}\nglow = {}, {}, {}\nfinish_contrast = {}\nfinish_saturation = {}\nfinish_warmth = {}\nvignette = {}\nchromatic = {}\n",
            self.exposure,
            if self.tonemap { "filmic" } else { "off" },
            self.glow,
            self.glow_radius,
            self.glow_threshold,
            self.contrast,
            self.saturation,
            self.warmth,
            self.vignette,
            self.chromatic
        )
    }
}

const GAMMA: f32 = 2.2;

#[inline]
fn luma(c: [f32; 3]) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

#[inline]
fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The filmic curve (an approximation of the ACES film response): 0 stays
/// 0, white becomes 0.8, light far above white approaches 1.
#[inline]
fn filmic(x: f32) -> f32 {
    let x = x.max(0.0);
    (x * (2.51 * x + 0.03) / (x * (2.43 * x + 0.59) + 0.14)).clamp(0.0, 1.0)
}

/// Applies `f` to `img` (RGB, 0..1 for white, more for brighter light;
/// `w` x `h` at `origin` in a picture of `full` size: the vignette and the
/// colour fringes go by the whole picture's middle).  `threads` share the
/// work.
pub fn apply(img: &mut [f32], w: usize, h: usize, origin: [i32; 2], full: [usize; 2], f: &Finish, threads: usize) {
    if f.is_neutral() || w == 0 || h == 0 {
        return;
    }
    let threads = threads.clamp(1, h);
    let rows = h.div_ceil(threads);
    let gain = 2f32.powf(f.exposure);
    // into linear light, the exposure
    par_rows(img, w, rows, |_, row| {
        for v in row.iter_mut() {
            *v = v.max(0.0).powf(GAMMA) * gain;
        }
    });
    let diag = ((full[0] * full[0] + full[1] * full[1]) as f32).sqrt();
    if f.glow > 0.0 {
        let glow = glow_of(img, w, h, f, diag, threads);
        for (v, g) in img.iter_mut().zip(&glow) {
            *v += g * f.glow;
        }
    }
    let (cx, cy) = (full[0] as f32 * 0.5 - origin[0] as f32, full[1] as f32 * 0.5 - origin[1] as f32);
    // the colour fringes: red a little outwards, blue a little inwards
    let shifted = (f.chromatic > 0.0).then(|| img.to_vec());
    let shift = f.chromatic * 0.006;
    let half_diag = diag * 0.5;
    let warm = f.warmth.clamp(-1.0, 1.0) * 0.12;
    par_rows(img, w, rows, |y0, rows_px| {
        for (i, px) in rows_px.chunks_exact_mut(3).enumerate() {
            let (x, y) = ((i % w) as f32 + 0.5, (y0 + i / w) as f32 + 0.5);
            let mut c = [px[0], px[1], px[2]];
            if let Some(src) = &shifted {
                let (dx, dy) = (x - cx, y - cy);
                c[0] = sample(src, w, h, cx + dx * (1.0 + shift) - 0.5, cy + dy * (1.0 + shift) - 0.5, 0);
                c[2] = sample(src, w, h, cx + dx * (1.0 - shift) - 0.5, cy + dy * (1.0 - shift) - 0.5, 2);
            }
            if f.tonemap {
                c = c.map(filmic);
            }
            // back to the picture's scale
            c = c.map(|v| v.max(0.0).powf(1.0 / GAMMA));
            if f.contrast != 0.0 {
                let k = f.contrast.clamp(-1.0, 1.0);
                c = c.map(|v| if v <= 1.0 { v + k * (smoothstep(0.0, 1.0, v) - v) * 1.6 } else { v });
            }
            if f.saturation != 1.0 {
                let l = luma(c);
                c = c.map(|v| l + (v - l) * f.saturation.max(0.0));
            }
            if warm != 0.0 {
                c[0] *= 1.0 + warm;
                c[2] *= 1.0 - warm;
            }
            if f.vignette > 0.0 {
                let r = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt() / half_diag;
                let k = 1.0 - f.vignette.clamp(0.0, 1.0) * smoothstep(0.35, 1.05, r);
                c = c.map(|v| v * k);
            }
            px.copy_from_slice(&c.map(|v| v.max(0.0)));
        }
    });
}

/// [`apply`] on a whole 8 bit RGB picture (Monte Carlo's).
pub fn apply_rgb8(rgb: &mut [u8], w: usize, h: usize, f: Option<&Finish>) {
    let Some(f) = f.filter(|f| !f.is_neutral()) else { return };
    let mut v: Vec<f32> = rgb.iter().map(|&c| c as f32 / 255.0).collect();
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    apply(&mut v, w, h, [0, 0], [w, h], f, threads);
    for (o, x) in rgb.iter_mut().zip(v) {
        *o = (x * 255.0).round().clamp(0.0, 255.0) as u8;
    }
}

/// Runs `job(first row, rows)` on parts of `img` in threads.
fn par_rows(img: &mut [f32], w: usize, rows: usize, job: impl Fn(usize, &mut [f32]) + Sync) {
    std::thread::scope(|s| {
        for (k, part) in img.chunks_mut(rows * w * 3).enumerate() {
            let job = &job;
            s.spawn(move || job(k * rows, part));
        }
    });
}

/// The colour channel `ch` at (x, y) (pixel centres at whole numbers),
/// between the pixels.
fn sample(img: &[f32], w: usize, h: usize, x: f32, y: f32, ch: usize) -> f32 {
    let x = x.clamp(0.0, (w - 1) as f32);
    let y = y.clamp(0.0, (h - 1) as f32);
    let (x0, y0) = (x as usize, y as usize);
    let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let p = |xx: usize, yy: usize| img[(yy * w + xx) * 3 + ch];
    let a = p(x0, y0) + (p(x1, y0) - p(x0, y0)) * fx;
    let b = p(x0, y1) + (p(x1, y1) - p(x0, y1)) * fx;
    a + (b - a) * fy
}

/// The glow of the light above the threshold (linear light, the picture's
/// size): blurred on a smaller copy, a wide and a near part.
fn glow_of(img: &[f32], w: usize, h: usize, f: &Finish, diag: f32, threads: usize) -> Vec<f32> {
    let radius = (f.glow_radius.max(0.0) * diag).max(1.0);
    // the smaller copy: about 6 of its pixels per radius
    let ds = ((radius / 6.0).floor() as usize).clamp(1, 64);
    let (sw, sh) = (w.div_ceil(ds), h.div_ceil(ds));
    let thr = f.glow_threshold.max(0.0).powf(GAMMA);
    let mut small = vec![0f32; sw * sh * 3];
    for sy in 0..sh {
        for sx in 0..sw {
            let mut acc = [0f32; 3];
            let mut n = 0f32;
            for y in sy * ds..((sy + 1) * ds).min(h) {
                for x in sx * ds..((sx + 1) * ds).min(w) {
                    let o = (y * w + x) * 3;
                    let c = [img[o], img[o + 1], img[o + 2]];
                    let l = luma(c);
                    if l > thr {
                        let k = (l - thr) / l;
                        for (a, v) in acc.iter_mut().zip(c) {
                            *a += v * k;
                        }
                    }
                    n += 1.0;
                }
            }
            let o = (sy * sw + sx) * 3;
            small[o..o + 3].copy_from_slice(&acc.map(|a| a / n.max(1.0)));
        }
    }
    let sigma = radius / ds as f32;
    let wide = blur(&small, sw, sh, sigma * 0.5, threads);
    let near = blur(&small, sw, sh, sigma * 0.15, threads);
    let soft: Vec<f32> = wide.iter().zip(&near).map(|(a, b)| a * 0.6 + b * 0.4).collect();
    // back to the picture's size, between the small pixels
    let mut out = vec![0f32; w * h * 3];
    let inv = 1.0 / ds as f32;
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = ((x as f32 + 0.5) * inv - 0.5, (y as f32 + 0.5) * inv - 0.5);
            let o = (y * w + x) * 3;
            for ch in 0..3 {
                out[o + ch] = sample(&soft, sw, sh, fx, fy, ch);
            }
        }
    }
    out
}

/// A Gaussian blur of `img` (RGB, `w` x `h`) with the spread `sigma`
/// pixels, the edges continued.
fn blur(img: &[f32], w: usize, h: usize, sigma: f32, threads: usize) -> Vec<f32> {
    let sigma = sigma.max(0.5);
    let r = (sigma * 3.0).ceil() as isize;
    let k: Vec<f32> = (-r..=r).map(|i| (-(i * i) as f32 / (2.0 * sigma * sigma)).exp()).collect();
    let ks: f32 = k.iter().sum();
    let k: Vec<f32> = k.iter().map(|v| v / ks).collect();
    let pass = |src: &[f32], horizontal: bool| -> Vec<f32> {
        let mut dst = vec![0f32; src.len()];
        let rows = h.div_ceil(threads.clamp(1, h.max(1)));
        std::thread::scope(|s| {
            for (part, chunk) in dst.chunks_mut(rows * w * 3).enumerate() {
                let k = &k;
                s.spawn(move || {
                    for (i, o) in chunk.chunks_exact_mut(3).enumerate() {
                        let (x, y) = ((i % w) as isize, (part * rows + i / w) as isize);
                        let mut acc = [0f32; 3];
                        for (j, kv) in k.iter().enumerate() {
                            let d = j as isize - r;
                            let (xx, yy) = if horizontal { ((x + d).clamp(0, w as isize - 1), y) } else { (x, (y + d).clamp(0, h as isize - 1)) };
                            let q = (yy as usize * w + xx as usize) * 3;
                            for c in 0..3 {
                                acc[c] += src[q + c] * kv;
                            }
                        }
                        o.copy_from_slice(&acc);
                    }
                });
            }
        });
        dst
    };
    let t = pass(img, true);
    pass(&t, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_changes_nothing_and_filmic_rolls_off() {
        let mut img: Vec<f32> = (0..48).map(|i| i as f32 / 47.0).collect();
        let orig = img.clone();
        apply(&mut img, 4, 4, [0, 0], [4, 4], &Finish::default(), 2);
        assert_eq!(img, orig);
        assert_eq!(filmic(0.0), 0.0);
        assert!((filmic(1.0) - 0.8).abs() < 0.01);
        assert!(filmic(10.0) > 0.98 && filmic(10.0) <= 1.0);
        assert!((1..100).all(|i| filmic(i as f32 * 0.1) >= filmic((i - 1) as f32 * 0.1)));
    }

    #[test]
    fn glow_spreads_bright_light_and_vignette_darkens_corners() {
        let (w, h) = (64, 48);
        let mut img = vec![0.1f32; w * h * 3];
        let c = (24 * w + 32) * 3;
        img[c..c + 3].copy_from_slice(&[4.0, 4.0, 4.0]);
        let f = Finish { glow: 1.0, glow_radius: 0.08, glow_threshold: 0.8, ..Default::default() };
        apply(&mut img, w, h, [0, 0], [w, h], &f, 3);
        // a pixel beside the light got brighter, a far one stayed
        let near = img[(24 * w + 36) * 3];
        let far = img[(2 * w + 2) * 3];
        assert!(near > 0.12, "{near}");
        assert!((far - 0.1).abs() < 0.01, "{far}");
        let mut img = vec![0.5f32; w * h * 3];
        apply(&mut img, w, h, [0, 0], [w, h], &Finish { vignette: 1.0, ..Default::default() }, 2);
        assert!(img[0] < 0.3 && (img[(24 * w + 32) * 3] - 0.5).abs() < 1e-3);
        // a tile of a bigger picture: its vignette by the whole picture's middle
        let mut tile = vec![0.5f32; 16 * 16 * 3];
        apply(&mut tile, 16, 16, [24, 16], [64, 48], &Finish { vignette: 1.0, ..Default::default() }, 1);
        assert!((tile[(8 * 16 + 8) * 3] - 0.5).abs() < 1e-3);
    }
}
