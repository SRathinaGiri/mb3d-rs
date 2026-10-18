//! Per-pixel calculation result (`TsiLight5`), the "G-buffer" that the
//! ray marcher produces and the painter consumes.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SiLight {
    /// Surface normal in view space, scaled to +-32767.
    pub normal: [i16; 3],
    /// `RoughZposFine` + `Zpos` as one cardinal: bits 8..31 = z position
    /// (24 bit), bits 0..7 = roughness.  `Zpos` (bits 16..31) >= 32768
    /// marks background.
    pub zpos_fine: u32,
    /// DE step count (dynamic fog), 10 bit, + hard shadow bits.
    pub shadow: u16,
    pub amb_shadow: u16,
    /// Smoothed iteration gradient for colouring, high bit = inside colour.
    pub si_gradient: u16,
    /// Orbit trap colouring value.
    pub otrap: u16,
}

impl Default for SiLight {
    fn default() -> Self {
        SiLight {
            normal: [0; 3],
            zpos_fine: 32768 << 16,
            shadow: 0,
            amb_shadow: 0,
            si_gradient: 0,
            otrap: 0,
        }
    }
}

impl SiLight {
    /// The 16 bit `Zpos` word.
    #[inline]
    pub fn zpos(&self) -> u32 {
        self.zpos_fine >> 16
    }

    #[inline]
    pub fn is_background(&self) -> bool {
        self.zpos() >= 32768
    }
}

/// The bytes of a pixel in a file (as in a .m3i: the normal, the depth,
/// the shadow, ambient shadow, gradient and orbit trap words).
pub const REC_BYTES: usize = 18;

/// The pixels as [`REC_BYTES`] each, little endian.
pub fn pack(px: &[SiLight]) -> Vec<u8> {
    let mut d = Vec::with_capacity(px.len() * REC_BYTES);
    for s in px {
        for n in s.normal {
            d.extend_from_slice(&n.to_le_bytes());
        }
        d.extend_from_slice(&s.zpos_fine.to_le_bytes());
        for w in [s.shadow, s.amb_shadow, s.si_gradient, s.otrap] {
            d.extend_from_slice(&w.to_le_bytes());
        }
    }
    d
}

/// The pixels of [`pack`]ed bytes.
pub fn unpack(d: &[u8]) -> Vec<SiLight> {
    d.chunks_exact(REC_BYTES)
        .map(|b| {
            let h = |k: usize| u16::from_le_bytes([b[k], b[k + 1]]);
            SiLight {
                normal: [h(0) as i16, h(2) as i16, h(4) as i16],
                zpos_fine: u32::from_le_bytes([b[6], b[7], b[8], b[9]]),
                shadow: h(10),
                amb_shadow: h(12),
                si_gradient: h(14),
                otrap: h(16),
            }
        })
        .collect()
}

/// `DoubleImageSize` (DoubleSize.pas): the G-buffer at twice the width and
/// height; the new pixels are the averages of their 2 or 4 neighbours.
pub fn double_size(g: &[SiLight], w: usize, h: usize) -> Vec<SiLight> {
    let w2 = w * 2;
    let mut o = vec![SiLight::default(); w2 * h * 2];
    for y in 0..h {
        for x in 0..w {
            o[(y * w2 + x) * 2] = g[y * w + x];
        }
    }
    let avg = |p: &[SiLight]| -> SiLight {
        let n = p.len() as i32;
        let nrm = |k: usize| (p.iter().map(|s| s.normal[k] as i32).sum::<i32>() / n) as i16;
        let b: u32 = p.iter().map(|s| s.zpos_fine & 0xFF).sum::<u32>() / n as u32;
        let z: u32 = p.iter().map(|s| s.zpos_fine >> 8).sum::<u32>();
        // (sum shl 6) for 4, (sum shl 7) for 2 values: the average shl 8
        let shift = if n == 4 { 6 } else { 7 };
        let zf = b | ((z << shift) & 0xFFFF_FF00);
        let a16 = |f: fn(&SiLight) -> u16| (p.iter().map(|s| f(s) as u32).sum::<u32>() / n as u32) as u16;
        SiLight {
            normal: [nrm(0), nrm(1), nrm(2)],
            zpos_fine: zf,
            shadow: a16(|s| s.shadow),
            amb_shadow: a16(|s| s.amb_shadow),
            si_gradient: a16(|s| s.si_gradient),
            otrap: a16(|s| s.otrap),
        }
    };
    for y in 0..h {
        let y3 = if y == h - 1 { y } else { y + 1 };
        for x in 0..w {
            let x2 = if x == w - 1 { x } else { x + 1 };
            let p1 = o[(2 * y) * w2 + 2 * x];
            let p2 = o[(2 * y) * w2 + 2 * x2];
            let p3 = o[(2 * y3) * w2 + 2 * x];
            let p4 = o[(2 * y3) * w2 + 2 * x2];
            o[(2 * y + 1) * w2 + 2 * x + 1] = avg(&[p1, p2, p3, p4]);
            o[(2 * y) * w2 + 2 * x + 1] = avg(&[p1, p2]);
            o[(2 * y + 1) * w2 + 2 * x] = avg(&[p1, p3]);
        }
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_size_averages() {
        let mut a = SiLight::default();
        a.zpos_fine = 100 << 8;
        a.normal = [100, 0, -100];
        let mut b = a;
        b.zpos_fine = 200 << 8;
        b.normal = [300, 0, -300];
        let d = double_size(&[a, b], 2, 1);
        assert_eq!(d.len(), 8);
        assert_eq!(d[1].zpos_fine >> 8, 150);
        assert_eq!(d[1].normal, [200, 0, -200]);
        assert_eq!(d[2], b);
    }
}

