//! Surface detail (not in MB3D): a fine relief and colour variation on the
//! fractal's surface from 3D noise fixed to the fractal, finer than the
//! calculation's detail: the relief tilts the surface's normal (bump
//! mapping) where the light is calculated (the painting, the reflections,
//! Monte Carlo), the colour variation darkens and lightens its colour.

/// The surface detail (`Scene::detail`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Detail {
    /// how strongly the relief tilts the surface (0 = none, 1 strong)
    pub strength: f32,
    /// the size of the relief's features (fractal units)
    pub size: f64,
    /// how much the finer features add (0.2 smooth .. 0.8 grainy)
    pub roughness: f32,
    /// how much the colour varies with it (0 = not, 1 strong)
    pub color: f32,
    /// the kind of relief: 0 rough (soft noise), 1 stone (sharp ridges and
    /// cracks), 2 grain (fine and even)
    pub kind: u8,
}

impl Default for Detail {
    fn default() -> Self {
        Detail { strength: 0.5, size: 0.02, roughness: 0.5, color: 0.0, kind: 0 }
    }
}

/// The kinds' names as the scene keys write them.
pub const KINDS: &[&str] = &["rough", "stone", "grain"];

impl Detail {
    fn octaves(&self) -> u32 {
        if self.kind == 2 {
            3
        } else {
            5
        }
    }

    /// The relief's height at `p` (fractal coordinates), about -1..1.
    pub fn height(&self, p: [f64; 3]) -> f64 {
        let s = 1.0 / self.size.max(1e-30);
        let mut q = p.map(|v| v * s);
        let g = self.roughness.clamp(0.05, 0.95) as f64;
        let (mut sum, mut norm, mut amp) = (0.0, 0.0, 1.0);
        for i in 0..self.octaves() {
            let mut n = noise3(q, i);
            if self.kind == 1 {
                // stone: sharp ridges (the noise's zero lines folded up)
                n = 1.0 - 2.0 * n.abs();
            }
            sum += n * amp;
            norm += amp;
            amp *= g;
            q = turn(q);
        }
        sum / norm
    }

    /// The unit normal `n` (fractal coordinates) tilted by the relief at
    /// `p`.
    pub fn bump(&self, n: [f64; 3], p: [f64; 3]) -> [f64; 3] {
        if self.strength <= 0.0 {
            return n;
        }
        let e = self.size.max(1e-30) * 0.03;
        let h0 = self.height(p);
        let mut g = [0.0; 3];
        for k in 0..3 {
            let mut q = p;
            q[k] += e;
            // (per feature size: a slope of 1 over a feature)
            g[k] = (self.height(q) - h0) / e * self.size;
        }
        let gn = g[0] * n[0] + g[1] * n[1] + g[2] * n[2];
        let k = self.strength as f64 * 0.35;
        let t = [0, 1, 2].map(|i| n[i] - k * (g[i] - gn * n[i]));
        let l = (t[0] * t[0] + t[1] * t[1] + t[2] * t[2]).sqrt();
        if l > 1e-30 {
            t.map(|v| v / l)
        } else {
            n
        }
    }

    /// The colour factor at `p` (1 = as it is).
    pub fn tint(&self, p: [f64; 3]) -> f32 {
        if self.color <= 0.0 {
            return 1.0;
        }
        // larger patches than the relief, and the relief's own shading
        let s = 1.0 / (self.size.max(1e-30) * 3.0);
        let q = p.map(|v| v * s + 17.3);
        let n = 0.65 * noise3(q, 11) + 0.35 * noise3(turn(q), 12);
        (1.0 + self.color.clamp(0.0, 1.0) as f64 * 0.6 * n).max(0.0) as f32
    }

    /// The scene keys of these settings.
    pub fn to_text(&self) -> String {
        format!("detail = {}, {}, {}, {}\ndetail_color = {}\n", KINDS[self.kind as usize % KINDS.len()], self.strength, self.size, self.roughness, self.color)
    }
}

/// The next octave's place: about twice as fine, turned (so the octaves'
/// grids do not line up).
#[inline]
fn turn(q: [f64; 3]) -> [f64; 3] {
    [
        1.6 * q[0] - 1.2 * q[1] + 0.31,
        1.2 * q[0] + 1.28 * q[1] - 0.96 * q[2],
        0.72 * q[1] + 1.6 * q[2] + 0.17,
    ]
}

#[inline]
fn hash(x: i32, y: i32, z: i32, seed: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x8DA6_B343) ^ (y as u32).wrapping_mul(0xD816_3841) ^ (z as u32).wrapping_mul(0xCB1A_B31F) ^ seed.wrapping_mul(0x1656_67B1);
    h ^= h >> 16;
    h = h.wrapping_mul(0x7FEB_352D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846C_A68B);
    h ^ (h >> 16)
}

/// The gradient of a lattice point: one of the 12 edge directions of a cube.
#[inline]
fn grad(h: u32, x: f64, y: f64, z: f64) -> f64 {
    match h % 12 {
        0 => x + y,
        1 => -x + y,
        2 => x - y,
        3 => -x - y,
        4 => x + z,
        5 => -x + z,
        6 => x - z,
        7 => -x - z,
        8 => y + z,
        9 => -y + z,
        10 => y - z,
        _ => -y - z,
    }
}

/// 3D gradient noise, about -1..1.
pub fn noise3(p: [f64; 3], seed: u32) -> f64 {
    let f = p.map(f64::floor);
    let i = f.map(|v| v as i64 as i32);
    let (x, y, z) = (p[0] - f[0], p[1] - f[1], p[2] - f[2]);
    let fade = |t: f64| t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
    let (u, v, w) = (fade(x), fade(y), fade(z));
    let g = |dx: i32, dy: i32, dz: i32| grad(hash(i[0].wrapping_add(dx), i[1].wrapping_add(dy), i[2].wrapping_add(dz), seed), x - dx as f64, y - dy as f64, z - dz as f64);
    let lerp = |a: f64, b: f64, t: f64| a + (b - a) * t;
    let x00 = lerp(g(0, 0, 0), g(1, 0, 0), u);
    let x10 = lerp(g(0, 1, 0), g(1, 1, 0), u);
    let x01 = lerp(g(0, 0, 1), g(1, 0, 1), u);
    let x11 = lerp(g(0, 1, 1), g(1, 1, 1), u);
    lerp(lerp(x00, x10, v), lerp(x01, x11, v), w) * 0.95
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_is_bounded_and_continuous() {
        let (mut lo, mut hi) = (f64::MAX, f64::MIN);
        for i in 0..6000 {
            let p = [i as f64 * 0.137 - 300.0, (i % 53) as f64 * 0.29, (i % 7) as f64 * 0.41 - 1.0];
            let n = noise3(p, 3);
            lo = lo.min(n);
            hi = hi.max(n);
            assert!((noise3([p[0] + 1e-6, p[1], p[2]], 3) - n).abs() < 1e-4);
        }
        assert!(lo >= -1.01 && hi <= 1.01 && hi - lo > 1.0, "{lo} {hi}");
    }

    #[test]
    fn bump_keeps_a_unit_normal_near_the_old_one() {
        for kind in 0..3u8 {
            let d = Detail { kind, strength: 1.0, color: 1.0, ..Default::default() };
            let n = [0.0, 0.6, 0.8];
            let mut moved = 0;
            for i in 0..200 {
                let p = [i as f64 * 0.0037, 0.1, -0.2];
                let b = d.bump(n, p);
                let l = (b[0] * b[0] + b[1] * b[1] + b[2] * b[2]).sqrt();
                assert!((l - 1.0).abs() < 1e-9);
                let c = b[0] * n[0] + b[1] * n[1] + b[2] * n[2];
                assert!(c > 0.3, "kind {kind}: cos {c}");
                moved += (c < 0.999) as usize;
                assert!(d.tint(p) > 0.0);
            }
            assert!(moved > 100, "kind {kind}");
        }
        // no strength: the normal as it was
        let d = Detail { strength: 0.0, ..Default::default() };
        assert_eq!(d.bump([1.0, 0.0, 0.0], [0.3, 0.2, 0.1]), [1.0, 0.0, 0.0]);
    }
}
