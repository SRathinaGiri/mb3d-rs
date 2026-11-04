//! OpenEXR pictures: the colours in floating point (not clipped above
//! white), with the layers a compositing program needs: the opacity (A),
//! the depth (Z) and the surface normals (N.X, N.Y, N.Z).  Written here
//! without a library: one part, scan lines, ZIP compression (the zlib of
//! the PNG writer), colours, opacity and normals as half floats, the depth
//! as 32 bit floats.
//!
//! The colours are linear (as OpenEXR expects): a viewer that shows them
//! with the usual sRGB display curve shows the same picture as the PNG,
//! and the parts brighter than white keep their brightness.

use crate::calc::CalcParams;
use crate::gbuffer::SiLight;
use crate::scene::Scene;

/// The layers of a picture, `width` x `height`; each optional one is left
/// out of the file when None.
pub struct Layers {
    pub width: usize,
    pub height: usize,
    /// linear RGB, 3 per pixel
    pub rgb: Vec<f32>,
    /// the opacity: 1 on the object, 0 on the background
    pub alpha: Option<Vec<f32>>,
    /// the distance from the camera's start plane in fractal units (a very
    /// large value, 1e10, on the background, as Blender writes it)
    pub depth: Option<Vec<f32>>,
    /// the surface normal in camera space (x right, y down, z into the
    /// picture), 3 per pixel; 0 on the background
    pub normal: Option<Vec<f32>>,
}

/// The depth written for the background.
pub const BACKGROUND_DEPTH: f32 = 1e10;

/// A shown colour (1 = white, the picture's gamma applied) as a linear
/// value: the inverse of the sRGB display curve, continued above white.
pub fn linear(v: f32) -> f32 {
    if v <= 0.04045 {
        v.max(0.0) / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

/// The layers of a calculated picture (its G-buffer `gbuf`, `w` x `h`),
/// reduced by `n` (anti-aliasing, the picture scale; 1 = none): colours as
/// the mean of each n x n block, the opacity as the share of its object
/// pixels, the depth as the nearest of them and the normal as their mean.
pub fn layers(sc: &Scene, p: &CalcParams, gbuf: &[SiLight], w: usize, h: usize, n: usize) -> Layers {
    let shown = crate::render::paint_f32(sc, p, gbuf);
    let cam = crate::render::paint_camera(sc, p);
    let n = n.max(1);
    let (w2, h2) = (w / n, h / n);
    let mut rgb = vec![0f32; w2 * h2 * 3];
    let mut alpha = vec![0f32; w2 * h2];
    let mut depth = vec![BACKGROUND_DEPTH; w2 * h2];
    let mut normal = vec![0f32; w2 * h2 * 3];
    let inv = 1.0 / (n * n) as f32;
    for y in 0..h2 {
        for x in 0..w2 {
            let o = y * w2 + x;
            let (mut k, mut nrm) = (0u32, [0f32; 3]);
            for dy in 0..n {
                for dx in 0..n {
                    let i = (y * n + dy) * w + x * n + dx;
                    for c in 0..3 {
                        rgb[o * 3 + c] += linear(shown[i * 3 + c]) * inv;
                    }
                    let s = &gbuf[i];
                    if s.is_background() {
                        continue;
                    }
                    k += 1;
                    let z1 = (((8388352 - (s.zpos_fine >> 8) as i64) as f64 / cam.zc_mul + 1.0).powi(2) - 1.0) * cam.step_width / cam.zcorr;
                    depth[o] = depth[o].min(z1 as f32);
                    for c in 0..3 {
                        nrm[c] += s.normal[c] as f32 / 32767.0;
                    }
                }
            }
            alpha[o] = k as f32 * inv;
            let len = (nrm[0] * nrm[0] + nrm[1] * nrm[1] + nrm[2] * nrm[2]).sqrt();
            if len > 0.0 {
                for c in 0..3 {
                    normal[o * 3 + c] = nrm[c] / len;
                }
            }
        }
    }
    Layers { width: w2, height: h2, rgb, alpha: Some(alpha), depth: Some(depth), normal: Some(normal) }
}

/// An f32 as a half float (IEEE 754 binary16), rounded to nearest even;
/// too large values become infinity.
pub fn half(v: f32) -> u16 {
    let b = v.to_bits();
    let sign = ((b >> 16) & 0x8000) as u16;
    let exp = ((b >> 23) & 0xff) as i32;
    let man = b & 0x7f_ffff;
    if exp == 0xff {
        // infinity or NaN
        return sign | 0x7c00 | if man != 0 { 0x200 } else { 0 };
    }
    let e = exp - 127 + 15;
    if e >= 0x1f {
        return sign | 0x7c00;
    }
    if e <= 0 {
        // subnormal half (or zero)
        if e < -10 {
            return sign;
        }
        let m = man | 0x80_0000;
        let shift = (14 - e) as u32;
        let mut h = m >> shift;
        let rest = m & ((1 << shift) - 1);
        let halfway = 1 << (shift - 1);
        if rest > halfway || (rest == halfway && h & 1 == 1) {
            h += 1;
        }
        return sign | h as u16;
    }
    let mut h = ((e as u32) << 10) | (man >> 13);
    let rest = man & 0x1fff;
    if rest > 0x1000 || (rest == 0x1000 && h & 1 == 1) {
        h += 1; // may carry into the exponent, up to infinity: correct
    }
    sign | h as u16
}

const HALF: i32 = 1;
const FLOAT: i32 = 2;
/// scan lines per block of the ZIP compression
const ZIP_LINES: usize = 16;

/// One channel: its name and pixel type, and where its values are.
struct Chan<'a> {
    name: &'a str,
    kind: i32,
    data: &'a [f32],
    stride: usize,
    offset: usize,
}

fn attr(out: &mut Vec<u8>, name: &str, kind: &str, value: &[u8]) {
    out.extend(name.as_bytes());
    out.push(0);
    out.extend(kind.as_bytes());
    out.push(0);
    out.extend((value.len() as i32).to_le_bytes());
    out.extend(value);
}

/// The OpenEXR file of `l`.
pub fn encode(l: &Layers) -> Vec<u8> {
    let (w, h) = (l.width, l.height);
    // the channels in alphabetical order (as the format requires)
    let mut ch = Vec::new();
    if let Some(a) = &l.alpha {
        ch.push(Chan { name: "A", kind: HALF, data: a, stride: 1, offset: 0 });
    }
    ch.push(Chan { name: "B", kind: HALF, data: &l.rgb, stride: 3, offset: 2 });
    ch.push(Chan { name: "G", kind: HALF, data: &l.rgb, stride: 3, offset: 1 });
    if let Some(nm) = &l.normal {
        for (i, name) in ["N.X", "N.Y", "N.Z"].into_iter().enumerate() {
            ch.push(Chan { name, kind: HALF, data: nm, stride: 3, offset: i });
        }
    }
    ch.push(Chan { name: "R", kind: HALF, data: &l.rgb, stride: 3, offset: 0 });
    if let Some(z) = &l.depth {
        ch.push(Chan { name: "Z", kind: FLOAT, data: z, stride: 1, offset: 0 });
    }

    let mut out = Vec::new();
    out.extend(20000630u32.to_le_bytes()); // magic
    out.extend(2u32.to_le_bytes()); // version 2, single part scan lines
    let mut list = Vec::new();
    for c in &ch {
        list.extend(c.name.as_bytes());
        list.push(0);
        list.extend(c.kind.to_le_bytes());
        list.extend([0u8; 4]); // pLinear, reserved
        list.extend(1i32.to_le_bytes());
        list.extend(1i32.to_le_bytes());
    }
    list.push(0);
    attr(&mut out, "channels", "chlist", &list);
    attr(&mut out, "compression", "compression", &[3]); // ZIP
    let mut window = Vec::new();
    for v in [0i32, 0, w as i32 - 1, h as i32 - 1] {
        window.extend(v.to_le_bytes());
    }
    attr(&mut out, "dataWindow", "box2i", &window);
    attr(&mut out, "displayWindow", "box2i", &window);
    attr(&mut out, "lineOrder", "lineOrder", &[0]); // increasing y
    attr(&mut out, "pixelAspectRatio", "float", &1f32.to_le_bytes());
    attr(&mut out, "screenWindowCenter", "v2f", &[0u8; 8]);
    attr(&mut out, "screenWindowWidth", "float", &1f32.to_le_bytes());
    out.push(0); // end of the header

    // the blocks, compressed in parallel
    let blocks = h.div_ceil(ZIP_LINES);
    let mut data: Vec<Vec<u8>> = vec![Vec::new(); blocks];
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(blocks.max(1));
    let per = blocks.div_ceil(threads.max(1)).max(1);
    std::thread::scope(|s| {
        for (t, part) in data.chunks_mut(per).enumerate() {
            let ch = &ch;
            s.spawn(move || {
                for (k, slot) in part.iter_mut().enumerate() {
                    *slot = block(ch, w, h, (t * per + k) * ZIP_LINES);
                }
            });
        }
    });

    // the offset table, then the blocks (first line, size, data)
    let mut pos = (out.len() + blocks * 8) as u64;
    for d in &data {
        out.extend(pos.to_le_bytes());
        pos += 8 + d.len() as u64;
    }
    for (b, d) in data.iter().enumerate() {
        out.extend(((b * ZIP_LINES) as i32).to_le_bytes());
        out.extend((d.len() as i32).to_le_bytes());
        out.extend(d);
    }
    out
}

/// One block from line `y0`: the lines one after the other, each with its
/// channels one after the other; then ZIP: the bytes split into even and
/// odd ones, as differences, deflated (stored as they are when that is not
/// smaller).
fn block(ch: &[Chan], w: usize, h: usize, y0: usize) -> Vec<u8> {
    let mut raw = Vec::new();
    for y in y0..(y0 + ZIP_LINES).min(h) {
        for c in ch {
            for x in 0..w {
                let v = c.data[(y * w + x) * c.stride + c.offset];
                if c.kind == HALF {
                    raw.extend(half(v).to_le_bytes());
                } else {
                    raw.extend(v.to_le_bytes());
                }
            }
        }
    }
    let n = raw.len();
    let mut t: Vec<u8> = Vec::with_capacity(n);
    t.extend(raw.iter().step_by(2));
    t.extend(raw.iter().skip(1).step_by(2));
    let mut p = t[0];
    for v in t.iter_mut().skip(1) {
        let d = v.wrapping_sub(p).wrapping_add(128);
        p = *v;
        *v = d;
    }
    let z = crate::png::zlib(&t);
    if z.len() < n { z } else { raw }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_floats() {
        assert_eq!(half(0.0), 0);
        assert_eq!(half(-0.0), 0x8000);
        assert_eq!(half(1.0), 0x3c00);
        assert_eq!(half(-2.0), 0xc000);
        assert_eq!(half(0.5), 0x3800);
        assert_eq!(half(65504.0), 0x7bff);
        assert_eq!(half(70000.0), 0x7c00);
        assert_eq!(half(f32::INFINITY), 0x7c00);
        assert_eq!(half(5.960_464_5e-8), 1); // smallest subnormal
        assert_eq!(half(1.0 / 3.0), 0x3555);
    }

    #[test]
    fn linear_is_the_inverse_of_srgb() {
        assert!(linear(0.0).abs() < 1e-9);
        assert!((linear(1.0) - 1.0).abs() < 1e-6);
        assert!((linear(0.5) - 0.214).abs() < 1e-3);
        assert!(linear(2.0) > 4.0);
    }
}
