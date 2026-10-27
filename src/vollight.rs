//! Volumetric light, ported from `maps/Maps.pas` (`TVolumetricLightMap`,
//! `MakeVolumicLightMapThreads`, `TCalcVolumetricLightMapThread`,
//! `VolLightMapPos`, `GetVolLightMapVec`) and `TMandCalcThread.DoDynFog`.
//!
//! Before the main calculation a shadow map of one light is calculated by
//! ray marching from the light: a cube map (6 sides) for a positional light,
//! or one side (an orthographic depth map along the light direction) for a
//! global light.  During the main ray march the lit fraction of each step is
//! summed up; the painter shows it with the dynamic fog colours.

use crate::calc::{CalcParams, Marcher};
use crate::math::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VolLightParams {
    /// header light index 0..5
    pub light: usize,
    /// map size: -7..7 in 20 % steps (`bVolLightNr shr 4 - 2`)
    pub map_size: i32,
}

#[derive(Clone, Debug)]
pub struct VolLightMap {
    pub cube_size: i32,
    pub half_size: i32,
    pub size_factor: f32,
    pub stretch: f32,
    pub min_distance: f32,
    pub is_pos_light: bool,
    pub light_pos: Vec3,
    /// rows: x, y and depth axis of the (global light) map
    pub rot: [[f32; 3]; 3],
    pub sides: Vec<Vec<f32>>,
    /// `VLmul`, `VLstepmul`
    pub vl_mul: f32,
    pub vl_step_mul: f32,
}

impl VolLightMap {
    /// `VolLightMapPosPas`: is the (absolute) position lit by a global light?
    #[inline]
    pub fn pos_lit(&self, vd: &Vec3) -> bool {
        let v = [
            (vd[0] - self.light_pos[0]) as f32,
            (vd[1] - self.light_pos[1]) as f32,
            (vd[2] - self.light_pos[2]) as f32,
        ];
        let r = &self.rot;
        let t = |k: usize| v[0] * r[k][0] + v[1] * r[k][1] + v[2] * r[k][2];
        let cs = self.cube_size;
        let x = ((t(0) * self.stretch).round_ties_even() as i32 + self.half_size).clamp(0, cs - 1);
        let y = ((t(1) * self.stretch).round_ties_even() as i32 + self.half_size).clamp(0, cs - 1);
        self.sides[0][(x + y * cs) as usize] > t(2)
    }

    /// `GetVolLightMapVecPas`: distance to the first surface from the
    /// positional light in direction `vd`.
    #[inline]
    pub fn vec(&self, vd: [f32; 3]) -> f32 {
        let (a0, a1, a2) = (vd[0].abs(), vd[1].abs(), vd[2].abs());
        let (cs, x, y);
        if a0 >= a1 && a0 >= a2 {
            cs = if vd[0] > 0.0 { 0 } else { 1 };
            let f = self.size_factor / vd[0];
            x = (vd[1] * f).round_ties_even() as i32;
            y = (vd[2] * f).round_ties_even() as i32;
        } else if a0 >= a1 || a1 < a2 {
            cs = if vd[2] > 0.0 { 4 } else { 5 };
            let f = self.size_factor / vd[2];
            x = (vd[0] * f).round_ties_even() as i32;
            y = (vd[1] * f).round_ties_even() as i32;
        } else {
            cs = if vd[1] > 0.0 { 2 } else { 3 };
            let f = self.size_factor / vd[1];
            x = (vd[0] * f).round_ties_even() as i32;
            y = (vd[2] * f).round_ties_even() as i32;
        }
        let h = self.half_size;
        let i = ((x + h).clamp(0, self.cube_size - 1) + (y + h).clamp(0, self.cube_size - 1) * self.cube_size) as usize;
        self.sides[cs][i]
    }

    /// The volumetric part of `DoDynFog`: light collected on the segment
    /// from `end - dir * last_step` to `end` (`dir` = view ray per step).
    pub fn integrate(&self, p: &CalcParams, end: &Vec3, dir: &Vec3, last_step: f64, mzz: f64, step_count: &mut f32) {
        let d2 = p.step_width * p.step_width;
        let mut v = [end[0] - dir[0] * last_step, end[1] - dir[1] * last_step, end[2] - dir[2] * last_step];
        let s1_0 = ((1.0 + mzz * p.de_stop_factor as f64) * self.vl_step_mul as f64) as f32;
        let mut s1 = s1_0;
        let mut st = last_step as f32;
        let mut guard = 0;
        loop {
            if s1 > st {
                s1 = st;
            }
            add_weight(&mut v, dir, s1 as f64);
            st -= s1;
            if self.is_pos_light {
                let vs = [
                    (v[0] - self.light_pos[0]) as f32,
                    (v[1] - self.light_pos[1]) as f32,
                    (v[2] - self.light_pos[2]) as f32,
                ];
                let d1 = vs[0] * vs[0] + vs[1] * vs[1] + vs[2] * vs[2];
                let m = self.vec(vs);
                if m * m > d1 {
                    *step_count += (self.vl_mul as f64 * s1 as f64 / (d1 as f64 + d2)) as f32;
                }
            } else if self.pos_lit(&v) {
                *step_count += self.vl_mul * s1;
            }
            guard += 1;
            if st < 0.01 || guard > 100_000 {
                break;
            }
        }
    }
}

/// `MakeVolumicLightMapThreads` + `TCalcVolumetricLightMapThread.Execute`.
///
/// `ln` is the light vector of the selected light as the painter uses it:
/// the position relative to the scene middle for a positional light, the
/// view space direction towards the light for a global light.
#[allow(clippy::too_many_arguments)]
pub fn build(
    p: &CalcParams,
    mid: [f64; 3],
    ln: [f32; 3],
    positional: bool,
    amplitude: f32,
    vp: &VolLightParams,
    hs_max_len_mul: f32,
    threads: usize,
) -> VolLightMap {
    let mut ystart = p.ystart;
    if p.optic != crate::scene::CameraOptic::Panorama {
        add_weight(&mut ystart, &p.vgrads[0], p.width as f64 * 0.5);
        add_weight(&mut ystart, &p.vgrads[1], p.height as f64 * 0.5);
    }
    let mut s = p.width.max(p.height) as f64;
    let sft = 1.0 + 0.2 * vp.map_size as f64;
    let sw = p.step_width;
    let mut rot = [[0f32; 3]; 3];
    let (cube_size, side_count, stretch, min_distance, light_pos);
    if positional {
        light_pos = [mid[0] + ln[0] as f64, mid[1] + ln[1] as f64, mid[2] + ln[2] as f64];
        side_count = 6;
        cube_size = (s * sft * 0.25).round_ties_even() as i32 | 1;
        stretch = 1.0f32;
        min_distance = 0.0f32;
    } else {
        let mut dtmp = p.zend.min(s * 16.0);
        let sm = dtmp.max(s) / s;
        s *= sm.powf(0.36);
        dtmp *= 0.5;
        side_count = 1;
        cube_size = (s * sft * 0.625).round_ties_even() as i32 | 1;
        let vz = p.vgrads[2];
        let k = p.zz_stmit_dif / sw + dtmp;
        light_pos = [mid[0] + vz[0] * k, mid[1] + vz[1] * k, mid[2] + vz[2] * k];
        // direction from the light into the scene
        let sv = rotate_vector_reverse(&[ln[0] as f64, ln[1] as f64, ln[2] as f64], &p.vgrads);
        let sv = normalize([-sv[0], -sv[1], -sv[2]]);
        // MakeOrthoVecs
        let o = if sv[0].abs() > 0.1 {
            let d = 1.0 / (sv[0] * sv[0] + sv[2] * sv[2]).sqrt();
            [sv[2] * d, 0.0, -sv[0] * d]
        } else {
            let d = 1.0 / (sv[1] * sv[1] + sv[2] * sv[2]).sqrt();
            [0.0, -sv[2] * d, sv[1] * d]
        };
        let o2 = [o[1] * sv[2] - o[2] * sv[1], o[2] * sv[0] - o[0] * sv[2], o[0] * sv[1] - o[1] * sv[0]];
        for k in 0..3 {
            rot[0][k] = o[k] as f32;
            rot[1][k] = o2[k] as f32;
            rot[2][k] = sv[k] as f32;
        }
        let d = sw * dtmp * 2.0 * sft.max(1.0).sqrt().sqrt();
        stretch = (cube_size as f64 / (d * 1.75)) as f32;
        min_distance = (-d * hs_max_len_mul as f64) as f32;
    }
    let half_size = cube_size / 2;
    let cs = cube_size as usize;
    let mut map = VolLightMap {
        cube_size,
        half_size,
        size_factor: (cube_size - 1) as f32 * 0.5,
        stretch,
        min_distance,
        is_pos_light: positional,
        light_pos,
        rot,
        sides: Vec::new(),
        vl_mul: (if positional { 1000.0 } else { 300.0 }) * amplitude / p.width as f32,
        vl_step_mul: (1.0 / (1.0 + 0.1 * vp.map_size as f64)).sqrt() as f32,
    };

    let max_dist_pos = hs_max_len_mul as f64 * ((cube_size * 3) as f64).max(p.zend * 0.5);
    let max_dist_glob = (min_distance as f64).abs() * 2.0 / sw;
    let sgl = 1.0 / stretch as f64;
    let hs = half_size;
    let threads = threads.max(1);
    let m = &map;
    let n = side_count * cs * cs;
    // the texel `i` of all sides
    let texel = |mr: &mut Marcher, i: usize| -> f32 {
        let side = i / (cs * cs);
        let x = (i % cs) as i32 - hs;
        let y = (i / cs % cs) as i32 - hs;
        let (start, dir, max_dist) = if positional {
            let (xf, yf, h) = (x as f64, y as f64, hs as f64);
            let v = match side {
                0 => [h, xf, yf],
                1 => [-h, -xf, -yf],
                2 => [xf, h, yf],
                3 => [-xf, -h, -yf],
                4 => [xf, yf, h],
                _ => [-xf, -yf, -h],
            };
            (m.light_pos, normalize(v), max_dist_pos)
        } else {
            let r = &m.rot;
            let dir = [r[2][0] as f64, r[2][1] as f64, r[2][2] as f64];
            let mut c = m.light_pos;
            add_weight(&mut c, &dir, min_distance as f64);
            for k in 0..3 {
                c[k] += r[0][k] as f64 * x as f64 * sgl + r[1][k] as f64 * y as f64 * sgl;
            }
            (c, dir, max_dist_glob)
        };
        let dist = mr.vl_trace(start, dir, &ystart, max_dist);
        (dist * sw) as f32 + min_distance
    };
    // the texels still free: [card, cpu) (the card takes them from the
    // front, the CPU threads from the back, as in the reflection pass)
    let free = std::sync::Mutex::new((0usize, n));
    let card_done = std::sync::atomic::AtomicBool::new(true);
    let mut data = vec![0f32; n];
    #[allow(unused_mut)]
    let mut card: Option<Result<Vec<f32>, String>> = None;
    std::thread::scope(|sc| {
        // the graphics card when it can (the same march: the shader's
        // M_VLMAP)
        // (it joins once its shader is compiled)
        #[cfg(feature = "gpu")]
        let card_thread = crate::gpu::vol_map_prepare(p).map(|ready| {
            card_done.store(false, std::sync::atomic::Ordering::SeqCst);
            let max_dist = if positional { max_dist_pos } else { max_dist_glob };
            let (free, card_done) = (&free, &card_done);
            // (the card of this thread's picture)
            let card = crate::gpu::card_here();
            sc.spawn(move || crate::gpu::as_card(card, || {
                use std::sync::atomic::Ordering::SeqCst;
                let left = || {
                    let f = free.lock().unwrap();
                    f.0 < f.1
                };
                while !ready.load(SeqCst) && left() {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                let r = left().then(|| {
                    let claim = |want: usize| {
                        let mut f = free.lock().unwrap();
                        f.0 = f.0.max(want.min(f.1));
                        f.0
                    };
                    crate::gpu::vol_map(p, m, max_dist, &claim)
                });
                card_done.store(true, SeqCst);
                r
            }))
        });
        const PIECE: usize = 64;
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                let (free, card_done, texel) = (&free, &card_done, &texel);
                sc.spawn(move || {
                    let mut mr = Marcher::new(p, 1);
                    let mut out = Vec::new();
                    loop {
                        let (lo, hi) = {
                            let mut f = free.lock().unwrap();
                            let hi = f.1;
                            f.1 = hi.saturating_sub(PIECE).max(f.0);
                            (f.1, hi)
                        };
                        if lo == hi {
                            if card_done.load(std::sync::atomic::Ordering::SeqCst) {
                                break;
                            }
                            // the card has the rest for now
                            std::thread::sleep(std::time::Duration::from_millis(1));
                            continue;
                        }
                        for i in lo..hi {
                            out.push((i, texel(&mut mr, i)));
                        }
                    }
                    out
                })
            })
            .collect();
        for w in workers {
            for (i, v) in w.join().unwrap() {
                data[i] = v;
            }
        }
        #[cfg(feature = "gpu")]
        if let Some(t) = card_thread {
            card = t.join().unwrap();
        }
    });
    let n_card = free.lock().unwrap().0;
    match card {
        Some(Ok(v)) => data[..n_card].copy_from_slice(&v[..n_card]),
        Some(Err(e)) => {
            // the card's part here
            eprintln!("volumetric light map on the GPU: {e} (on the CPU)");
            let mut mr = Marcher::new(p, 1);
            for i in 0..n_card {
                data[i] = texel(&mut mr, i);
            }
        }
        None => {}
    }
    if std::env::var_os("MB3D_GPU_TIMING").is_some() && n_card > 0 {
        eprintln!("  volumetric light map: {} % of the texels on the card", n_card * 100 / n.max(1));
    }
    map.sides = data.chunks(cs * cs).map(|c| c.to_vec()).collect();
    map
}

impl<'a> Marcher<'a> {
    /// The ray march of `TCalcVolumetricLightMapThread.Execute`: distance
    /// (in steps) from `start` along unit `dir` to the first surface.
    pub fn vl_trace(&mut self, start: Vec3, dir: Vec3, ystart: &Vec3, max_dist: f64) -> f64 {
        let p = self.p;
        let sw = p.step_width;
        self.it.c = start;
        let mut rsf = 1f64;
        let mut zz = len(&sub(start, *ystart)) / sw;
        self.ms_de_stop = (p.de_stop as f64 * (1.0 + zz * p.de_stop_factor as f64)) as f32;
        let vz = p.vgrads[2];
        let zzmul = dot(&vz, &dir) / (sqr_len(&vz) * sqr_len(&dir)).sqrt().max(1e-300);
        let mut ds = self.calc_de();
        let mut dist = 0f64;
        let dirs = scale(dir, sw);
        loop {
            let last_de = ds;
            let step = f64::min(
                f64::max(0.11, (ds - p.ms_de_sub as f64 * self.ms_de_stop as f64) * p.z_step_div as f64 * rsf),
                (f32::max(self.ms_de_stop, 0.4) * p.mh04zsd) as f64,
            );
            dist += step;
            add_weight(&mut self.it.c, &dirs, step);
            zz += step * zzmul;
            self.ms_de_stop = (p.de_stop as f64 * (1.0 + zz.abs() * p.de_stop_factor as f64)) as f32;
            ds = self.calc_de();
            if self.it.it_result >= self.max_its_result || ds <= self.ms_de_stop as f64 {
                break;
            }
            if ds > last_de + step {
                ds = last_de + step;
            }
            rsf = if last_de > ds + 1e-30 {
                let t = step / (last_de - ds);
                if t < 1.0 {
                    t.max(0.5)
                } else {
                    1.0
                }
            } else {
                1.0
            };
            if dist >= max_dist {
                dist = max_dist * 1000.0;
                break;
            }
        }
        dist
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn vlight_encoding_round_trips_roughly() {
        use crate::lighting::{convert_vlight, encode_vlight};
        for v in [0.0f32, 5.0, 127.0, 128.0, 1000.0, 9000.0, 16383.0] {
            let d = convert_vlight(encode_vlight(v)) as f32;
            assert!((d - v).abs() <= v / 64.0 + 0.5, "{v} -> {d}");
        }
        assert_eq!(convert_vlight(encode_vlight(1e9)), convert_vlight(encode_vlight(16383.0)));
    }
}
