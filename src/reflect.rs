//! Reflections and transparency of the normal renderer (`CalcSR.pas`) and
//! the normals calculated from the z-buffer (`NormalsOnZbuf`,
//! ImageProcess.pas), two of MB3D's automatic post processing steps.
//!
//! The reflection pass runs after painting: for every object pixel the
//! view ray is reflected (or refracted into transparent material, with
//! Fresnel reflection and absorption), marched to the next surface, which
//! gets its own normal, colour, hard shadow and DE ambient occlusion, and
//! is lit like a painted pixel (`CalcPixelColorSvec(Trans)`), up to
//! `SRreflectioncount` times.  The specular colour of the palette is the
//! amount of reflected light, its alpha the transparency.

use crate::calc::{CalcParams, Marcher};
use crate::gbuffer::SiLight;
use crate::lighting::{LightVals, PaintCamera, Plv, ShadeOpts};
use crate::math::*;
use crate::scene::{CameraOptic, Scene};

type S3 = [f32; 3];

#[inline]
fn s3(v: &Vec3) -> S3 {
    [v[0] as f32, v[1] as f32, v[2] as f32]
}
#[inline]
fn add3(a: S3, b: S3) -> S3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
#[inline]
fn sub3(a: S3, b: S3) -> S3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
#[inline]
fn mul3(a: S3, b: S3) -> S3 {
    [a[0] * b[0], a[1] * b[1], a[2] * b[2]]
}
#[inline]
fn sc3(a: S3, s: f32) -> S3 {
    [a[0] * s, a[1] * s, a[2] * s]
}
#[inline]
fn dot3(a: S3, b: S3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
#[inline]
fn luma(v: S3) -> f32 {
    (v[0] * 0.3 + v[1] * 0.59 + v[2] * 0.11).max(0.0)
}
#[inline]
fn spow(v: S3, e: f32) -> S3 {
    [v[0].powf(e), v[1].powf(e), v[2].powf(e)]
}
#[inline]
fn lin2(sv1: S3, sv2: S3, w1: f32) -> S3 {
    [sv2[0] + w1 * (sv1[0] - sv2[0]), sv2[1] + w1 * (sv1[1] - sv2[1]), sv2[2] + w1 * (sv1[2] - sv2[2])]
}
#[inline]
fn norm_to(n: f64, v: Vec3) -> Vec3 {
    let d = n / (v[0] * v[0] + v[1] * v[1] + v[2] * v[2] + 1e-100).sqrt();
    [v[0] * d, v[1] * d, v[2] * d]
}
fn normal_of(si: &SiLight) -> Vec3 {
    [si.normal[0] as f64 / 32767.0, si.normal[1] as f64 / 32767.0, si.normal[2] as f64 / 32767.0]
}

// ---------------------------------------------------------------------------
// normals on the z-buffer

/// `GetDEstopFactor`: growth of the DE stop with the distance
pub fn de_stop_factor(sc: &Scene) -> f64 {
    let sw = sc.step_width();
    let ze = ((sc.z_end - sc.z_start) / sw).max(1e-16);
    let x1 = if sc.optic == CameraOptic::Panorama {
        0.01 * sc.height as f64 / (0.01f64.sin() * std::f64::consts::PI)
    } else {
        0.01 * sc.height as f64 / (0.01f64.sin() * (sc.fov_y.to_radians()).max(1.0 / 65535.0))
    };
    let x2 = sw * (x1 + ze) / x1;
    (x2 - sw) / (sw * ze.max(1.0 / 6.0))
}

/// `NormalsOnZbuf`: smoother normals from the positions of the neighbour
/// pixels (MB3D's "Normals on Z-buffer" post processing).
pub fn normals_on_zbuf(sc: &Scene, p: &CalcParams, gbuf: &mut [SiLight]) {
    let [left, top, w, h] = p.rect;
    let (w, h) = (w as usize, h as usize);
    if w == 0 || h == 0 {
        return;
    }
    let sw = p.step_width as f32;
    let planar = sc.optic as i32;
    let (aspect, sfov) = if sc.optic == CameraOptic::Panorama {
        (2.0f32, std::f32::consts::PI)
    } else {
        (sc.width as f32 / sc.height as f32, sc.fov_y.to_radians() as f32)
    };
    let zt = (sfov as f64 * 0.5).clamp(0.01, 1.5);
    let pl_optic_z = (zt.cos() * zt / zt.sin()) as f32;
    let dcorr = p.step_width / p.zcorr;
    let zc_inv = 1.0 / p.zc_mul;
    let dsf = de_stop_factor(sc) as f32;
    let (fw, fh) = (sc.width as f32, sc.height as f32);
    // CalcViewVec (without the stereo offset)
    let view_vec = |xp: f32, yp: f32| -> S3 {
        let cx = (0.5 - xp) * sfov * aspect;
        let cy = (yp - 0.5) * sfov;
        match planar {
            1 => {
                let v = [-cx, cy, pl_optic_z];
                sc3(v, 1.0 / dot3(v, v).sqrt())
            }
            2 => [-(cx.sin()) * cy.cos(), cy.sin(), cy.cos() * cx.cos()],
            _ => {
                let v = [-(cx.sin()), cy.sin(), cy.cos() * cx.cos()];
                sc3(v, 1.0 / dot3(v, v).sqrt())
            }
        }
    };
    let valid = |si: &SiLight| si.zpos() < 32768 && si.si_gradient < 32768;
    let dist = |si: &SiLight| -> f32 {
        let zf = (si.zpos_fine >> 8) as f64;
        ((((8388352.0 - zf) * zc_inv + 1.0).powi(2) - 1.0) * dcorr) as f32
    };
    const INV: f32 = -1e20;
    let w1 = w + 1;
    let mut buf = vec![[0f32; 3]; w1 * 3];
    let pos = |si: &SiLight, c: usize, yrow: f32, ypos: f32| -> S3 {
        let xp = [if planar != 2 { (c as i32 + left) as f32 * sw } else { 0.0 }, if planar != 2 { yrow } else { 0.0 }, 0.0];
        let v = view_vec((c as i32 + 1 + left) as f32 / fw, ypos);
        add3(sc3(v, dist(si)), xp)
    };
    // the first two rows
    for y in 1..=2usize {
        let r = y - 1;
        for x in 0..w {
            buf[x + y * w1] = if r < h && valid(&gbuf[r * w + x]) {
                pos(&gbuf[r * w + x], x, y as f32 * sw, (r as i32 + top) as f32 / fh)
            } else {
                [INV, 0.0, 0.0]
            };
        }
        buf[w + y * w1][0] = INV;
    }
    for x in 0..=w {
        buf[x][0] = INV;
    }
    let rec = |a: S3, b: S3, n: f32| -> S3 {
        let n = n * n;
        let w1 = 1.0 / (dot3(a, a) + n);
        let w2 = 1.0 / (dot3(b, b) + n);
        let wm = 1.0 / (w1 + w2);
        add3(sc3(a, w1 * wm), sc3(b, w2 * wm))
    };
    for y in 1..=h {
        let r = y - 1;
        for x in 0..w {
            if buf[x + w1][0] > -1e19 {
                let si = gbuf[r * w + x];
                let s0 = sw + dist(&si) * dsf;
                let (up, mid, down) = (buf[x], buf[x + w1], buf[x + 2 * w1]);
                let svy = if up[0] < -1e19 {
                    if down[0] > -1e19 {
                        sub3(mid, down)
                    } else {
                        [0.0, 1.0, 0.0]
                    }
                } else if down[0] > -1e19 {
                    rec(sub3(up, mid), sub3(mid, down), s0)
                } else {
                    sub3(up, mid)
                };
                let (lft, rgt) = (buf[x + w], buf[x + 1 + w1]);
                let svx = if lft[0] < -1e19 {
                    if rgt[0] > -1e19 {
                        sub3(mid, rgt)
                    } else {
                        [1.0, 0.0, 0.0]
                    }
                } else if rgt[0] > -1e19 {
                    rec(sub3(lft, mid), sub3(mid, rgt), s0)
                } else {
                    sub3(lft, mid)
                };
                // SVecToNormals(SVectorCross(svY, svX))
                let c = [svy[1] * svx[2] - svy[2] * svx[1], svy[2] * svx[0] - svy[0] * svx[2], svy[0] * svx[1] - svy[1] * svx[0]];
                let d = 32767.0 / ((c[0] as f64).powi(2) + (c[1] as f64).powi(2) + (c[2] as f64).powi(2) + 1e-100).sqrt();
                let n = &mut gbuf[r * w + x].normal;
                for k in 0..3 {
                    n[k] = (c[k] as f64 * d).round_ties_even().clamp(-32768.0, 32767.0) as i16;
                }
            }
        }
        for x in 0..w {
            buf[x] = buf[x + w1];
            buf[x + w1] = buf[x + 2 * w1];
        }
        let nr = r + 2;
        for x in 0..w {
            buf[x + 2 * w1] = if y < h.saturating_sub(1) && nr < h && valid(&gbuf[nr * w + x]) {
                pos(&gbuf[nr * w + x], x, (y + 2) as f32 * sw + top as f32 * sw, (y as i32 + 1 + top) as f32 / fh)
            } else {
                [INV, 0.0, 0.0]
            };
        }
    }
}

// ---------------------------------------------------------------------------
// reflections

struct SrCtx<'a> {
    p: &'a CalcParams,
    lv: &'a LightVals,
    cam: &'a PaintCamera,
    mid: Vec3,
    /// `HSvecs` per entry of `lv.lights`
    hs_vecs: Vec<Vec3>,
    max_reflections: i32,
    sr_light_amount: f32,
    trans_di_const: f32,
    calc_trans: bool,
    only_difs: bool,
    absorption: f32,
    absorp_coeff: f32,
    light_scattering_mul: f32,
    back_dist: f32,
    /// hard shadows: (lights as indices into `lv.lights` with their header
    /// index, soft, soft radius, max length multiplier)
    hs: Option<(Vec<(usize, usize)>, bool, f32, f32)>,
    /// DE ambient occlusion: (max length, first step random)
    ao: Option<(f32, bool)>,
    vol: bool,
}

struct SrTracer<'a, 'c> {
    c: &'c SrCtx<'a>,
    m: Marcher<'a>,
    /// the G-buffer value of the pixel (`srPsiLight`)
    sr_si: SiLight,
    normal: Vec3,
    t_amb: S3,
    plv: Plv,
    light_z: Vec<(f32, f32)>,
    calc_trans_r: bool,
    trans_flip_inside: bool,
    scale_amb_diff_down: bool,
    rnd: i32,
    guard: u32,
    /// MB3D_REFLECT_DEBUG=x,y: print the bounces of that pixel
    dbg: bool,
}

/// The pixel named by `MB3D_REFLECT_DEBUG` ("x,y").
fn debug_pixel() -> Option<(i32, i32)> {
    let v = std::env::var("MB3D_REFLECT_DEBUG").ok()?;
    let (a, b) = v.split_once(',')?;
    Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
}

impl<'a, 'c> SrTracer<'a, 'c> {
    #[inline]
    fn update_de_stop(&mut self, zz: f64) {
        let p = self.c.p;
        self.m.ms_de_stop = (p.de_stop as f64 * (1.0 + zz.abs() * p.de_stop_factor as f64)) as f32;
    }

    #[inline]
    fn toggle_inside(&mut self) {
        self.m.inside = !self.m.inside;
        self.m.calc_inside = !self.m.calc_inside;
    }

    #[inline]
    fn advance(&mut self, dir: &Vec3, d: f64) {
        add_weight(&mut self.m.it.c, dir, d);
    }

    fn rand(&mut self) -> f64 {
        self.rnd = self.rnd.wrapping_mul(0x343FD).wrapping_add(0x269EC3);
        ((self.rnd as u32 >> 8) & 0x7FFFFF) as f64 / 0x7FFFFF as f64
    }

    /// `minLengthToCutPlane`
    fn min_length_to_cut_plane(&self, len: &mut f64, plane: &mut usize, limit: f64, pos: &Vec3, v: &Vec3) {
        let p = self.c.p;
        *plane = 0;
        for k in 0..3 {
            if (p.cut_options >> k) & 1 != 0 && v[k].abs() > 1e-20 {
                let t = (p.cut_pos[k] - pos[k]) / v[k];
                if t > limit && t < *len {
                    *len = t;
                    *plane = k + 1;
                }
            }
        }
    }

    /// `CalcLightZPos`: depth of positional lights seen along the ray
    fn calc_light_z_pos(&mut self) {
        let p = self.c.p;
        let sw = p.step_width as f32;
        let av = self.plv.abs_view;
        let cp = self.plv.cam_pos;
        for (i, l) in self.c.lv.lights.iter().enumerate() {
            if !l.positional {
                continue;
            }
            let d = if av[0].abs() > 0.5 {
                (l.ln[0] - cp[0]) / av[0]
            } else if av[1].abs() > 0.5 {
                (l.ln[1] - cp[1]) / av[1]
            } else {
                (l.ln[2] - cp[2]) / av[2]
            };
            let z = d.max(0.0);
            let lp = 8388352.0 - p.zc_mul * (((z / sw) as f64 * p.zcorr + 1.0).sqrt() - 1.0);
            self.light_z[i] = (z, ((lp / 256.0) as f32).clamp(0.0, 32767.0));
        }
    }

    /// `AddLight`: light scattered inside transparent material between the
    /// start of the ray and the object position
    fn add_light(&self, dif: S3) -> S3 {
        let mut res = [0f32; 3];
        let s3w = (self.c.p.step_width as f32).powi(2);
        let (cam, obj) = (self.plv.cam_pos, self.plv.obj_pos);
        for l in &self.c.lv.lights {
            let stmp = if l.positional {
                let sv = sub3(obj, cam);
                let sv2 = sub3(l.ln, cam);
                let sv3 = sub3(l.ln, obj);
                let d2 = dot3(sv2, sv2) + s3w;
                let d3 = dot3(sv3, sv3) + s3w;
                let st = dot3(sv2, sv);
                if st <= 0.0 {
                    0.25 / d3 + 1.0 / d2
                } else {
                    let st2 = dot3(sv, sv);
                    if st2 <= st {
                        0.25 / d2 + 1.0 / d3
                    } else {
                        let q = sub3(l.ln, add3(cam, sc3(sv, st / st2)));
                        1.0 / (dot3(q, q) + s3w) + 0.25 / d3.max(d2)
                    }
                }
            } else {
                1.0
            };
            for k in 0..3 {
                res[k] += l.col[k] * stmp;
            }
        }
        mul3(dif, res)
    }

    fn shade(&self, si: &SiLight, inside: Option<(S3, f32)>) -> crate::lighting::Shaded {
        let o = ShadeOpts {
            scale_amb_diff_down: if self.scale_amb_diff_down { Some(self.c.sr_light_amount) } else { None },
            inside_trans: inside,
            light_z: Some(&self.light_z),
        };
        self.c.lv.shade(si, &self.plv, self.c.cam, &o)
    }

    /// `CalcHS` / `CalcHSsoft` at the current position for the G-buffer
    /// value `si` (its normal)
    fn calc_hs(&mut self, si: &mut SiLight) {
        let Some((lights, soft, soft_radius, hs_mul)) = &self.c.hs else { return };
        let p = self.c.p;
        let sw = p.step_width;
        let zz = self.m.mzz.abs() as f32;
        let ic = self.m.it.c;
        let max_lhs = ((p.width + p.height) as f32 * 0.6 * (1.0 + 0.5 * zz.min(p.zend as f32 * 0.4) * (p.fov_y as f32).max(0.0) / p.height as f32) * hs_mul) as f64;
        let nvec = rotate_vector_reverse(&normal_of(si), &p.vgrads);
        let march = |me: &mut Self, hsvec: &Vec3, dt1: &mut f64, zz2: &mut f64, zz2mul: f64, zr: Option<(f32, f64)>| -> f32 {
            let mut rsfd = 1f32;
            let mut zrsoft = 1f32;
            let dmaxl = *dt1;
            let mut d = me.m.calc_de();
            let mut guard = 0;
            loop {
                guard += 1;
                let last_de = d;
                let st = ((d * p.z_step_div as f64 * rsfd as f64) as f32).min(me.m.ms_de_stop.max(0.4) * p.mh04zsd) as f64;
                *dt1 -= st;
                me.advance(hsvec, -st);
                *zz2 += st * zz2mul;
                me.update_de_stop(*zz2);
                d = me.m.calc_de();
                if let Some((zrs_mul, max_lhs)) = zr {
                    let r = ((dmaxl - *dt1) / max_lhs) as f32;
                    let r8 = (r * r) * (r * r);
                    zrsoft = zrsoft.min(((d - me.m.ms_de_stop as f64) * zrs_mul as f64 / (dmaxl - *dt1 + 0.11)) as f32 + r8 * r8);
                }
                if me.m.it.it_result >= me.m.max_its_result || d < me.m.ms_de_stop as f64 {
                    break;
                }
                if d > last_de + st {
                    d = last_de + st;
                }
                rsfd = if last_de > d + 1e-30 {
                    let t = (st / (last_de - d)) as f32;
                    if t < 1.0 {
                        t.max(0.5)
                    } else {
                        1.0
                    }
                } else {
                    1.0
                };
                if *dt1 < 0.0 || guard > 1_000_000 {
                    break;
                }
            }
            zrsoft
        };
        let cut_len = |me: &Self, v: &Vec3, len: &mut f64| {
            for k in 0..3 {
                if (p.cut_options >> k) & 1 != 0 && v[k].abs() > 1e-20 {
                    let t = (me.m.it.c[k] - p.cut_pos[k]) / v[k];
                    if t > 0.0 && t < *len {
                        *len = t;
                    }
                }
            }
        };
        if *soft {
            // CalcHSsoft: the last selected light
            let Some(&(li, _)) = lights.last() else { return };
            let l = &self.c.lv.lights[li];
            let zrs_mul = if l.positional {
                if l.visible & 6 == 2 {
                    70.0 / soft_radius
                } else {
                    40.0 / soft_radius
                }
            } else {
                80.0 / soft_radius
            };
            si.shadow |= 0xFC00;
            let mut zz2 = self.m.mzz;
            self.update_de_stop(zz as f64);
            let mut dmaxl = max_lhs;
            let mut hsvec = self.c.hs_vecs[li];
            if l.positional {
                let v = sub(add(s3d(l.ln), self.c.mid), self.m.it.c);
                let d = sqr_len(&v);
                if d > (l.lmax_l * hs_mul) as f64 {
                    return;
                }
                if d < (dmaxl * sw).powi(2) {
                    dmaxl = d.sqrt() / sw;
                }
                hsvec = norm_to(-sw, v);
            }
            let zz2mul = -dot(&hsvec, &self.m.vfov) / (sqr_len(&hsvec) * sqr_len(&self.m.vfov)).sqrt();
            if p.cut_options != 0 {
                cut_len(self, &hsvec, &mut dmaxl);
            }
            if dmaxl > 0.0 {
                if dot(&nvec, &hsvec) > 0.0 {
                    si.shadow &= 0x3FF;
                    return;
                }
                let mut dt1 = dmaxl;
                let zr = march(self, &hsvec, &mut dt1, &mut zz2, zz2mul, Some((zrs_mul, max_lhs)));
                si.shadow = (si.shadow & 0x3FF) | (((zr.clamp(0.0, 1.0) * 63.4).round_ties_even() as u16) << 10);
                self.m.it.c = ic;
            }
            return;
        }
        let mut mask = 0u16;
        for &(_, idx) in lights {
            mask |= 0x400 << idx;
        }
        si.shadow &= !mask;
        for &(li, idx) in lights {
            let l = &self.c.lv.lights[li];
            self.m.it.c = ic;
            let mut zz2 = self.m.mzz;
            self.update_de_stop(zz as f64);
            let mut dt1 = max_lhs;
            let mut hsvec = self.c.hs_vecs[li];
            if l.positional {
                let v = sub(add(s3d(l.ln), self.c.mid), self.m.it.c);
                let d = sqr_len(&v);
                if d > (l.lmax_l * hs_mul) as f64 {
                    si.shadow |= 0x400 << idx;
                    continue;
                }
                if d < (dt1 * sw).powi(2) {
                    dt1 = d.sqrt() / sw;
                }
                hsvec = norm_to(-sw, v);
            }
            let zz2mul = -dot(&hsvec, &self.m.vfov) / (sqr_len(&hsvec) * sqr_len(&self.m.vfov)).sqrt();
            if p.cut_options != 0 {
                cut_len(self, &hsvec, &mut dt1);
            }
            if dt1 > 0.0 {
                if dot(&nvec, &hsvec) > 0.0 {
                    si.shadow |= 0x400 << idx;
                    continue;
                }
                march(self, &hsvec, &mut dt1, &mut zz2, zz2mul, None);
                if dt1 > 0.0 {
                    si.shadow |= 0x400 << idx;
                }
            }
        }
        self.m.it.c = ic;
    }

    /// `CalcAmbShadowDEfor1pos`: DE ambient occlusion at the current
    /// position (no dithering)
    fn deao_1pos(&mut self, si: &mut SiLight, quality: i32, max_len: f32, first_random: bool) {
        si.amb_shadow = 0;
        if si.zpos() >= 32768 || si.si_gradient >= 32768 {
            return;
        }
        let p = self.c.p;
        let sw = p.step_width;
        self.m.it.calc_sit = false;
        let rot_v = normalise_matrix_to(sw, &p.vgrads);
        let s_max_d = max_len as f64 * 0.5 * ((p.height as f64).powi(2) + (p.width as f64).powi(2)).sqrt();
        let mut ms_de_stop = ((p.de_stop as f64 * (1.0 + self.m.mzz.abs() * p.de_stop_factor as f64)) as f32).min(1e6) as f64;
        let ic = self.m.it.c;
        let step_ao = ms_de_stop / p.de_stop as f64;
        let max_dist = s_max_d * step_ao.sqrt();
        let ray_dir = |ya: f64, za: f64| -> S3 {
            let (sy, cy) = ya.sin_cos();
            let (sz, cz) = za.sin_cos();
            [(sy * cz) as f32, (-sy * sz) as f32, (-cy) as f32]
        };
        let tau = std::f64::consts::TAU;
        let mut dirs: Vec<S3> = Vec::new();
        let (mut d_step_mul, d_min_a_dif, corr_w);
        if quality == 0 {
            let abr = 60f64.to_radians();
            for i in 0..3 {
                dirs.push(ray_dir(0.5 * abr, i as f64 * tau / 3.0));
            }
            d_step_mul = 1.8f64;
            d_min_a_dif = -1.0f32;
            corr_w = 0.3f32;
        } else {
            let abr = std::f64::consts::FRAC_PI_2 / (quality as f64 + 0.9);
            dirs.push(ray_dir(0.0, 0.0));
            for iy in 1..=quality {
                let n = (0.25f64.sin() * tau / abr).round_ties_even() as i32;
                for ix in 0..n {
                    dirs.push(ray_dir(abr * iy as f64, ix as f64 * tau / n as f64));
                }
            }
            d_step_mul = 1.0 + abr.sin();
            d_min_a_dif = (abr * 1.2).cos() as f32;
            corr_w = if quality == 1 { 0.2 } else { 0.1666 };
        }
        let inside = self.m.inside && !p.difs;
        if inside {
            d_step_mul *= 0.5;
        }
        let n = {
            let v = [si.normal[0] as f32, si.normal[1] as f32, si.normal[2] as f32];
            let l = dot3(v, v).sqrt().max(1e-30);
            [v[0] / l, v[1] / l, v[2] / l]
        };
        let rot_w = crate::deao::normal_matrix(n);
        if p.de_stop_factor != 0.0 {
            if !inside {
                ms_de_stop /= d_step_mul * d_step_mul;
            }
        } else {
            ms_de_stop = p.de_stop as f64 / (d_step_mul * d_step_mul);
        }
        let ray_count = dirs.len();
        let de_mul = if inside { 1.0 } else { (ray_count as f64 * 0.5).sqrt() };
        let abr = 1.2 / (1.0 / de_mul).clamp(-1.0, 1.0).asin();
        let md_d10 = 0.1 / (max_dist * de_mul);
        let mut min_ra = vec![0f32; ray_count];
        for (k, d) in dirs.iter().enumerate() {
            let v = [dot3(rot_w[0], *d), dot3(rot_w[1], *d), dot3(rot_w[2], *d)];
            let sv = rotate_vector_reverse(&[v[0] as f64, v[1] as f64, v[2] as f64], &rot_v);
            let mut dt1 = step_ao * d_step_mul;
            let mut s = 1f64;
            let mut end = false;
            let mut first = first_random;
            let mut guard = 0;
            loop {
                guard += 1;
                if first {
                    first = false;
                    dt1 *= self.rand() * 1.5 + 0.5;
                } else if dt1 > max_dist {
                    dt1 = max_dist;
                    end = true;
                }
                self.m.it.c = ic;
                add_weight(&mut self.m.it.c, &sv, dt1);
                if p.cut_options > 0 && self.m.at_cut_plane(dt1 * sw * 0.5) {
                    break;
                }
                let d2 = self.m.calc_de();
                if inside {
                    if d2 < ms_de_stop {
                        s = (dt1 / max_dist).powi(4);
                        break;
                    }
                    dt1 += dt1 * d_step_mul * 0.1 + ms_de_stop;
                } else {
                    s = s.min((d2 - ms_de_stop + dt1 * md_d10) / dt1);
                    if s < 0.02 {
                        break;
                    }
                    dt1 += d2.max(dt1 * d_step_mul);
                }
                if end || guard > 100_000 {
                    break;
                }
            }
            min_ra[k] = (s.max(0.0) * de_mul) as f32;
        }
        let mut amount = 0f64;
        if inside {
            amount = min_ra.iter().map(|&v| v as f64).sum();
        } else {
            for k in 0..ray_count {
                let mut s_add = 0f32;
                if min_ra[k] < 1.0 {
                    let max_add = 1.0 - min_ra[k];
                    for k2 in 0..ray_count {
                        if k2 == k {
                            continue;
                        }
                        let dt = dot3(dirs[k], dirs[k2]);
                        if dt > d_min_a_dif {
                            let overlap = min_ra[k2] - (dt.clamp(-1.0, 1.0).acos() as f64 * abr) as f32 + 1.0;
                            if overlap > 0.0 {
                                s_add += max_add.min(overlap) * corr_w;
                            }
                        }
                    }
                }
                amount += (s_add + min_ra[k]).min(1.0) as f64;
            }
        }
        si.amb_shadow = (16383.0 * (1.0 - amount / ray_count as f64)).round_ties_even().max(0.0) as u16;
        self.m.it.c = ic;
    }

    /// `CalcOpenAir`: the background seen by a reflected ray
    fn calc_open_air(&mut self, si: &mut SiLight, sd: &mut ([f32; 3], f32, [f32; 3]), t_absorb: &mut S3, calc_t: bool, zz: f32) {
        let lv = self.c.lv;
        let p = self.c.p;
        if (lv.far_fog || lv.sqr) && lv.s_depth.abs() > 1e-10 {
            let both = lv.far_fog && lv.sqr;
            let mut d = 1.0 - (1.0 + (self.sr_si.zpos() as i32 - 28000) as f32 * lv.s_depth).max(0.0);
            if d > 0.0 {
                d *= d;
                if both {
                    d *= d;
                }
            }
            let mut d2 = 1.0 - (1.0 - 28000.0 * lv.s_depth).max(0.0);
            if d2 > 0.0 {
                d2 *= d2;
                if both {
                    d2 *= d2;
                }
            }
            let mut e = d2 - d;
            if e > 0.0 {
                e = e.sqrt();
                if both {
                    e = e.sqrt();
                }
            }
            si.zpos_fine = ((60768.0 - e / lv.s_depth).clamp(32768.0, 65535.0).round_ties_even() as u32) << 16;
        } else {
            si.zpos_fine = 32768 << 16;
        }
        self.plv.z_pos = self.c.back_dist + p.zz_stmit_dif as f32;
        self.plv.zpos_dyn_fog = self.plv.z_pos;
        let mut inside = None;
        if calc_t && (self.trans_flip_inside ^ self.m.inside) {
            let tmp_amb = spow(sd.2, zz * self.c.absorption);
            let stmp = (1.0 - luma(tmp_amb)) * self.c.light_scattering_mul;
            *t_absorb = mul3(*t_absorb, tmp_amb);
            let al = self.add_light(sd.2);
            self.t_amb = add3(self.t_amb, sc3(mul3(al, *t_absorb), stmp));
            inside = Some((sd.2, self.c.absorp_coeff));
        }
        let sh = self.shade(si, inside);
        let tmp_amb = sh.light.map(|v| (v / 255.0).clamp(0.0, 8.0));
        self.t_amb = add3(self.t_amb, mul3(*t_absorb, tmp_amb));
    }

    /// The paint values along a reflected ray (the view, the positions of
    /// the lights and of the background, the depth colour).
    fn set_ray_paint(&mut self, new_vec: Vec3, zz2: f64) {
        let c = self.c;
        let p = c.p;
        let sw = p.step_width;
        // the paint values along the new ray
        let av = s3(&normalize(new_vec));
        self.plv.abs_view = av;
        self.plv.view = c.cam.to_view(av);
        self.calc_light_z_pos();
        let lv = c.lv;
        self.plv.zpos_dyn_fog = (zz2 * sw + p.zz_stmit_dif) as f32;
        let t = if lv.amb_rel_obj { self.plv.abs_view } else { self.plv.view };
        let fov_y = p.fov_y as f32;
        let mut ypos = t[1].clamp(-1.0, 1.0).asin() / fov_y + 0.5;
        if lv.bg.is_some() {
            let d = (p.fov_y / (2.0 * std::f64::consts::PI - p.fov_y).max(0.1)) as f32;
            if ypos > 1.0 {
                ypos = 1.0 + d * (1.0 - ypos);
                if ypos < 0.75 {
                    ypos = 1.5 - ypos;
                }
            } else if ypos < 0.0 {
                ypos = ypos.abs() * d;
                if ypos > 0.25 {
                    ypos = 0.5 - ypos;
                }
            }
        }
        self.plv.y_pos = ypos.clamp(0.0, 1.0);
        let dstep = (p.height as f64 / (p.fov_y * p.width as f64)) as f32;
        let mut xpos = t[0].atan2(t[2]) * dstep + 0.5;
        if lv.bg.is_some() {
            let d = 1.0 / (dstep * (2.0 * std::f32::consts::PI - 1.0 / dstep).max(0.1));
            if xpos > 1.0 {
                xpos = 1.0 + d * (1.0 - xpos);
            } else if xpos < 0.0 {
                xpos = xpos.abs() * d;
            }
        }
        self.plv.x_pos = xpos.clamp(0.0, 1.0);
        if !lv.amb_rel_obj {
            let mut s = self.plv.y_pos;
            if lv.depth_func == 1 {
                s *= s;
            } else if lv.depth_func != 0 {
                s = s.max(0.0).sqrt();
            }
            self.plv.dep_c = lin2(lv.depth_col2, lv.depth_col, s);
        }
    }

    /// `CalcRay`: follows the reflected (or refracted) ray from the current
    /// surface; `sd` = (specular colour, alpha, diffuse colour) of it.
    fn calc_ray(&mut self, mut zz: f64, sr_vec: Vec3, mut t_absorb: S3, mut sd: ([f32; 3], f32, [f32; 3]), rit: i32) {
        let c = self.c;
        let p = c.p;
        let sw = p.step_width;
        self.guard += 1;
        if self.guard > 10_000 {
            return;
        }
        self.normal = rotate_vector_reverse(&self.normal, &p.vgrads);
        self.calc_trans_r = if !c.calc_trans {
            false
        } else if !c.only_difs {
            true
        } else {
            self.m.calc_de();
            self.m.last_difs
        };
        let mut calc_t = self.calc_trans_r;
        let (mut tmp_loc, mut tmp_norm, mut tmp_absorb, mut tmp_sd, mut zz_tmp, mut spec_mul_t) =
            (self.m.it.c, self.normal, t_absorb, sd, zz, 0f64);
        loop {
            // lab1
            if t_absorb[0].abs() * 0.3 + t_absorb[1].abs() * 0.59 + t_absorb[2].abs() * 0.11 < 1e-4 {
                return;
            }
            let new_vec;
            loop {
                // lab2
                if calc_t {
                    let stmp = sd.0[0].max(sd.0[1]).max(sd.0[2]);
                    let dstep = if self.trans_flip_inside ^ self.m.inside { c.trans_di_const as f64 } else { 1.0 / c.trans_di_const as f64 };
                    let dl = len(&sr_vec);
                    let nl = len(&self.normal);
                    spec_mul_t = -dot(&sr_vec, &self.normal) / (dl * nl);
                    let zz2 = 1.0 - dstep * dstep * (1.0 - spec_mul_t * spec_mul_t);
                    if zz2 <= 0.0 {
                        // total internal reflection
                        let f = 1.0 - sd.1 + sd.1 * sd.1 / (stmp + 0.01);
                        sd.0 = sc3(sd.0, f);
                        calc_t = false;
                        continue;
                    }
                    tmp_loc = self.m.it.c;
                    tmp_norm = self.normal;
                    tmp_absorb = t_absorb;
                    tmp_sd = sd;
                    zz_tmp = zz;
                    let mut nv = sub(sr_vec, scale(self.normal, dl / (nl * dstep) * (zz2.sqrt() - dstep * spec_mul_t)));
                    nv = scale(nv, dl / len(&nv));
                    let dt = dot(&nv, &self.normal).abs() / (dl * nl);
                    spec_mul_t = spec_mul_t.abs();
                    if dt.abs() > 1e-16 {
                        let a = spec_mul_t;
                        spec_mul_t = (((a - dstep * dt) / (a + dstep * dt)).powi(2) + ((dt - dstep * a) / (dt + dstep * a)).powi(2)) * 0.5;
                    }
                    t_absorb = sc3(t_absorb, (1.0 - spec_mul_t * stmp as f64) as f32);
                    self.toggle_inside();
                    new_vec = nv;
                } else {
                    let n = self.normal;
                    new_vec = sub(sr_vec, scale(n, 2.0 * dot(&n, &sr_vec) / sqr_len(&n)));
                }
                break;
            }
            let zzplus = dot(&self.m.vfov, &new_vec) / (sqr_len(&self.m.vfov) * sqr_len(&new_vec) + 1e-100).sqrt();
            let mut max_l = p.zend;
            let mut cp = 0usize;
            if p.cut_options != 0 {
                let pos = self.m.it.c;
                self.min_length_to_cut_plane(&mut max_l, &mut cp, 1.0, &pos, &new_vec);
            }
            let last_zz = zz.abs() as f32;
            let mut zz2 = (p.de_stop as f64 * 0.25).min(1.0);
            zz += zz2 * zzplus;
            self.advance(&new_vec, zz2);
            self.update_de_stop(zz);
            self.plv.cam_pos = s3(&sub(self.m.it.c, c.mid));
            let mut open_air = true;
            let mut step_count = 0f32;
            let mut de_limited = true;
            let mut rlast_sw = 0f64;
            if self.sr_si.si_gradient > 32767 {
                zz2 = p.zend;
            } else {
                open_air = false;
                let mut rsfmul = 1f32;
                let mut dtmp = self.m.calc_de();
                rlast_sw = dtmp * p.z_step_div as f64;
                let mut guard = 0u32;
                if dtmp > 1e-10 {
                    loop {
                        let mut rlast_de = dtmp;
                        let mut dstep;
                        if c.vol {
                            dstep = dtmp;
                            self.m.mzz = zz.abs();
                            dstep = ((dstep - p.ms_de_sub as f64 * self.m.ms_de_stop as f64) * p.z_step_div as f64 * rsfmul as f64).max(0.11);
                            let s1 = (self.m.ms_de_stop.max(0.4) * p.mh04zsd) as f64;
                            if s1 < dstep {
                                dstep = s1;
                            }
                            if let Some(vl) = &p.vol {
                                let cc = self.m.it.c;
                                let vf = self.m.vfov;
                                vl.integrate(p, &cc, &vf, rlast_sw, self.m.mzz, &mut step_count);
                            }
                        } else {
                            dstep = ((dtmp - p.ms_de_sub as f64 * self.m.ms_de_stop as f64) * p.z_step_div as f64 * rsfmul as f64).max(0.11);
                            let s = (self.m.ms_de_stop.max(0.4) * p.mh04zsd) as f64;
                            let count = p.dfog_on_it == 0 || self.m.it.it_result == p.dfog_on_it as i32;
                            if s < dstep {
                                if count {
                                    step_count += (s / dstep) as f32;
                                }
                                dstep = s;
                            } else if count {
                                step_count += 1.0;
                            }
                        }
                        rlast_sw = dstep;
                        zz2 += dstep;
                        if zz2 > max_l {
                            if c.vol {
                                rlast_sw = max_l - zz2 + rlast_sw;
                                self.advance(&new_vec, rlast_sw);
                                if let Some(vl) = &p.vol {
                                    let cc = self.m.it.c;
                                    let vf = self.m.vfov;
                                    vl.integrate(p, &cc, &vf, rlast_sw, self.m.mzz, &mut step_count);
                                }
                                let _ = &mut rlast_de;
                            }
                            open_air = true;
                            break;
                        }
                        self.advance(&new_vec, dstep);
                        zz += dstep * zzplus;
                        self.update_de_stop(zz);
                        dtmp = self.m.calc_de();
                        if self.m.it.it_result >= self.m.max_its_result {
                            de_limited = false;
                            break;
                        }
                        if dtmp < self.m.ms_de_stop as f64 && self.m.it.it_result >= p.min_it {
                            break;
                        }
                        if dtmp > rlast_de + rlast_sw {
                            dtmp = rlast_de + rlast_sw;
                        }
                        rsfmul = if rlast_de > dtmp + 1e-10 {
                            let d = rlast_sw / (rlast_de - dtmp);
                            if d < 1.0 {
                                d.max(0.5) as f32
                            } else {
                                1.0
                            }
                        } else {
                            1.0
                        };
                        rlast_de = dtmp;
                        let _ = rlast_de;
                        guard += 1;
                        if guard > 1_000_000 {
                            open_air = true;
                            break;
                        }
                    }
                }
            }
            self.set_ray_paint(new_vec, zz2);
            let lv = c.lv;
            let mut si = SiLight::default();
            si.shadow = if c.vol {
                crate::lighting::encode_vlight(step_count)
            } else {
                if self.m.inside {
                    step_count *= 200.0 * p.de_stop / p.width as f32;
                }
                step_count.clamp(0.0, 1023.0).round_ties_even() as u16
            };
            if self.dbg {
                eprintln!("CPU bounce {rit}: new_vec {new_vec:?} open_air {open_air} zz {zz:.4} zz2 {zz2:.4} de_limited {de_limited} steps {step_count}");
            }
            if !open_air {
                if p.de_add_steps != 0 {
                    if de_limited {
                        let mut itmp = p.de_add_steps;
                        let mut ds = rlast_sw * -0.5;
                        let mut d = self.m.calc_de();
                        while (d - self.m.ms_de_stop as f64).abs() > 0.001 {
                            zz2 += ds;
                            zz += ds * zzplus;
                            self.advance(&new_vec, ds);
                            self.update_de_stop(zz);
                            itmp -= 1;
                            if itmp <= 0 {
                                break;
                            }
                            d = self.m.calc_de();
                            ds = if d < self.m.ms_de_stop as f64 { ds.abs() * -0.55 } else { ds.abs() * 0.55 };
                        }
                    } else {
                        let before = zz2;
                        let vf = self.m.vfov;
                        self.m.vfov = new_vec;
                        self.m.mzz = zz2;
                        self.m.bin_search_it();
                        zz2 = self.m.mzz;
                        self.m.vfov = vf;
                        zz += (zz2 - before) * zzplus;
                    }
                }
                self.m.mzz = zz.abs();
                if self.dbg {
                    eprintln!("  after the search: c {:?} zz2 {zz2:.4}", self.m.it.c);
                }
                let nn = if p.normals_on_de { self.m.calculate_normals(&mut si) } else { self.m.calculate_normals_on_smooth_it(&mut si) };
                if self.dbg {
                    eprintln!("  after the normals: c {:?}", self.m.it.c);
                }
                self.normal = normal_of(&si);
                let v = if de_limited {
                    32767.0 - (nn + p.d_col_plus + p.col_var_de_stop_mul * ((self.m.ms_de_stop as f64 * sw).ln() as f32)) * p.mcts_m
                } else {
                    32767.0 - nn * p.mcts_m
                };
                si.si_gradient = min_max_clip_15bit(v);
                if p.color_on_it != 0 {
                    self.m.do_color_on_it();
                }
                self.m.do_color(&mut si);
                let ds = if lv.far_fog || lv.sqr {
                    (((zz2 + last_zz as f64).powi(2) - last_zz as f64 + 1e-30).max(0.0)).sqrt()
                } else {
                    zz2
                };
                {
                    let v = (p.zc_mul * ((ds.max(0.0) * p.zcorr + 1.0).sqrt() - 1.0)).round_ties_even() as i64;
                    let z = (8388352 - v).clamp(0, 8388352) as u32;
                    let mut r = z << 8;
                    if p.sm_normals > 0 {
                        r |= ((self.m.s_roughness * 255.0).round_ties_even() as u32) & 0xFF;
                    }
                    si.zpos_fine = r;
                }
                let saved_de_stop = self.m.ms_de_stop;
                if c.hs.is_some() {
                    self.m.mzz = zz.abs();
                    self.calc_hs(&mut si);
                }
                self.plv.z_pos = (zz.abs() * sw + p.zz_stmit_dif) as f32;
                self.plv.obj_pos = s3(&sub(self.m.it.c, c.mid));
                if let Some((max_len, first)) = c.ao {
                    let q = match rit {
                        1 => 2,
                        2 => 1,
                        _ => 0,
                    };
                    self.m.mzz = zz.abs();
                    self.deao_1pos(&mut si, q, max_len, first);
                    if self.dbg {
                        eprintln!("  after the AO: c {:?}", self.m.it.c);
                    }
                    si.amb_shadow = ((si.amb_shadow as i32 * 8) / (8 + rit)) as u16;
                } else {
                    si.amb_shadow = 5000;
                }
                self.m.ms_de_stop = saved_de_stop;
                self.scale_amb_diff_down = self.calc_trans_r && rit < c.max_reflections;
                if calc_t {
                    t_absorb = sc3(t_absorb, sd.1);
                } else {
                    t_absorb = mul3(t_absorb, sd.0);
                }
                let mut inside = None;
                if self.trans_flip_inside ^ self.m.inside {
                    // absorption inside the material
                    let tmp_amb = spow(sd.2, (zz2.min(max_l) as f32) * c.absorption);
                    let tmp_spec = t_absorb;
                    let stmp = (1.0 - luma(tmp_amb)) * c.light_scattering_mul;
                    t_absorb = mul3(t_absorb, tmp_amb);
                    let al = self.add_light(sd.2);
                    self.t_amb = add3(self.t_amb, sc3(mul3(al, lin2(t_absorb, tmp_spec, 0.5)), stmp));
                    inside = Some((sd.2, c.absorp_coeff));
                }
                let sh = self.shade(&si, inside);
                if self.dbg {
                    eprintln!("  hit: pos {:?} si {si:?}\n  plv {:?}\n  light {:?} result {:?} spe {:?} {} dif {:?} t_absorb {t_absorb:?}", self.m.it.c, self.plv, sh.light, sh.result, sh.spe, sh.spe_a, sh.dif);
                }
                let tmp_amb = sh.light.map(|v| (v / 255.0).clamp(0.0, 8.0));
                sd = (sh.spe, sh.spe_a, sh.dif);
                self.t_amb = add3(self.t_amb, mul3(t_absorb, tmp_amb));
                if !calc_t || !(self.trans_flip_inside ^ self.m.inside) {
                    t_absorb = mul3(t_absorb, sh.result);
                }
                if rit < c.max_reflections {
                    t_absorb = sc3(t_absorb, c.sr_light_amount);
                    self.calc_ray(zz, new_vec, t_absorb, sd, rit + 1);
                }
            } else {
                si.zpos_fine = 32768 << 16;
                if calc_t {
                    t_absorb = sc3(t_absorb, sd.1);
                } else {
                    t_absorb = mul3(t_absorb, sd.0);
                }
                self.calc_open_air(&mut si, &mut sd, &mut t_absorb, calc_t, zz2.min(max_l) as f32);
                if self.dbg {
                    eprintln!("  open air: t_absorb {t_absorb:?} t_amb {:?} plv {:?}", self.t_amb, self.plv);
                }
            }
            if calc_t {
                self.m.it.c = tmp_loc;
                self.normal = tmp_norm;
                zz = zz_tmp;
                sd = tmp_sd;
                t_absorb = sc3(tmp_absorb, (spec_mul_t + (1.0 - spec_mul_t) * (1.0 - sd.1 as f64)) as f32);
                self.toggle_inside();
                calc_t = false;
                continue;
            }
            return;
        }
    }

    /// One pixel of `TSRCalcThread.Execute`: returns the new colour, or
    /// None to keep the painted one.
    fn pixel(&mut self, x: i32, y: i32, si: &SiLight) -> Option<[u8; 3]> {
        let c = self.c;
        let p = c.p;
        let sw = p.step_width;
        let lv = c.lv;
        if si.zpos() >= 32768 {
            return None;
        }
        self.sr_si = *si;
        self.guard = 0;
        let inside = if p.in_and_outside && si.otrap & 0x8000 != 0 { false } else { p.inside_rendering };
        self.m.inside = inside;
        self.m.calc_inside = inside;
        self.m.cafy = (y as f64 / p.height as f64 - 0.5) * p.fov_y;
        self.m.calc_vgrads_fov(x + 1);
        self.m.calc_start_pos(x, y);
        self.plv.cam_pos = s3(&sub(self.m.it.c, c.mid));
        let av = s3(&normalize(self.m.vfov));
        self.plv.abs_view = av;
        self.plv.view = c.cam.to_view(av);
        self.plv.y_pos = y as f32 / p.height as f32;
        self.plv.x_pos = (x + 1) as f32 / p.width as f32;
        {
            let ys = self.plv.y_pos;
            let s = match lv.depth_func {
                1 => ys * ys,
                0 => ys,
                _ => ys.max(0.0).sqrt(),
            };
            self.plv.dep_c = lin2(lv.depth_col2, lv.depth_col, s);
        }
        self.calc_light_z_pos();
        self.m.it.calc_sit = false;
        let zf = (si.zpos_fine >> 8) as f64;
        let zpos_of = |iz: f64| (((8388351.5 - iz) / p.zc_mul + 1.0).powi(2) - 1.0) / p.zcorr;
        self.m.mzz = zpos_of(zf);
        let vf = self.m.vfov;
        add_weight(&mut self.m.it.c, &vf, self.m.mzz);
        self.update_de_stop(self.m.mzz);
        if si.si_gradient < 32768 {
            let mut d = self.m.calc_de();
            if d < self.m.ms_de_stop as f64 * 0.5 {
                return None;
            }
            let de_limited = self.m.it.it_result < self.m.max_its_result || d <= self.m.ms_de_stop as f64;
            if de_limited {
                let dt1 = self.m.mzz - zpos_of(zf - 1.0);
                self.m.bin_search(&mut d, dt1);
            } else {
                self.m.bin_search_it();
            }
        } else {
            // on a cutting plane
            let mut dt1 = p.zend;
            let mut cp = 0;
            if p.cut_options != 0 {
                let pos = self.m.it.c;
                self.min_length_to_cut_plane(&mut dt1, &mut cp, -2.0, &pos, &vf);
            }
            if dt1.abs() > 1.0 {
                return None;
            }
            self.m.mzz += dt1;
            add_weight(&mut self.m.it.c, &vf, dt1);
            self.update_de_stop(self.m.mzz);
        }
        self.normal = normal_of(si);
        self.plv.z_pos = (self.m.mzz * sw + p.zz_stmit_dif) as f32;
        self.plv.obj_pos = s3(&sub(self.m.it.c, c.mid));
        self.plv.zpos_dyn_fog = self.plv.z_pos;
        self.calc_trans_r = c.calc_trans && (!c.only_difs || self.m.last_difs);
        self.scale_amb_diff_down = self.calc_trans_r;
        let sh = self.shade(si, None);
        self.t_amb = sh.light.map(|v| (v / 255.0).clamp(0.0, 8.0));
        let sv = sc3(sh.result, c.sr_light_amount);
        let mzz = self.m.mzz;
        if self.dbg {
            eprintln!("CPU surface: pos {:?} zz {mzz:.4} si {si:?}\n  plv {:?}\n  light {:?} result {:?} spe {:?} {} dif {:?}", self.m.it.c, self.plv, sh.light, sh.result, sh.spe, sh.spe_a, sh.dif);
        }
        self.calc_ray(mzz, vf, sv, (sh.spe, sh.spe_a, sh.dif), 1);
        let mut t = self.t_amb.map(|v| v.clamp(0.0, 1.0));
        if lv.sqr {
            t = t.map(|v| v.sqrt());
        }
        if lv.gamma_h != 0 {
            let g = if lv.gamma_h > 0 { t.map(|v| v.sqrt()) } else { t.map(|v| v * v) };
            t = add3(t, sc3(sub3(g, t), lv.s_gamma));
        }
        Some(t.map(|v| (v * 255.0).round().clamp(0.0, 255.0) as u8))
    }
}

#[inline]
fn s3d(v: S3) -> Vec3 {
    [v[0] as f64, v[1] as f64, v[2] as f64]
}

/// `CalcSRT`: the reflection pass over the painted image `rgb` of the
/// G-buffer `gbuf` (both covering `p.rect`).
#[allow(clippy::too_many_arguments)]
pub fn reflections(sc: &Scene, p: &CalcParams, gbuf: &[SiLight], rgb: &mut [u8], cam: &PaintCamera, lv: &LightVals, threads: usize) {
    let mc = &sc.mc;
    #[cfg(feature = "gpu")]
    if crate::gpu::enabled() && !mc.transparency && p.cut_options == 0 && p.vol.is_none() && !p.inside_rendering && !p.in_and_outside && crate::gpu::unsupported(p).is_none() {
        match reflections_gpu(sc, p, gbuf, rgb, cam, lv, threads) {
            Ok(()) => {
                let s = crate::gpu::last_status();
                if s.starts_with("GPU") && !s.contains("+reflections") {
                    crate::gpu::set_status(format!("{s} +reflections"));
                }
                return;
            }
            // the program ends (gpu::abort_all): no CPU pass either
            Err(e) if e == "cancelled" => return,
            Err(e) => eprintln!("reflections on the GPU failed, calculated on the CPU: {e}"),
        }
    }
    let mid = sc.stereo_mid();
    let m1 = normalise_matrix_to(1.0, &p.vgrads);
    let hs_vecs: Vec<Vec3> = lv
        .lights
        .iter()
        .map(|l| {
            if l.positional {
                add(s3d(l.ln), mid)
            } else {
                rotate_vector_reverse(&norm_to(-p.step_width, s3d(l.ln)), &m1)
            }
        })
        .collect();
    let hs = sc.shadows.map(|h| {
        let lights: Vec<(usize, usize)> =
            lv.lights.iter().enumerate().filter(|(_, l)| (h.lights >> l.idx) & 1 != 0).map(|(i, l)| (i, l.idx)).collect();
        (lights, h.soft, h.soft_radius.max(0.001), h.max_len_mul)
    });
    let ao = sc.ao.as_ref().map(|_| {
        let d = sc.deao.unwrap_or_default();
        (d.max_len, d.first_step_random)
    });
    let ctx = SrCtx {
        p,
        lv,
        cam,
        mid,
        hs_vecs,
        max_reflections: mc.reflection_depth as i32,
        sr_light_amount: mc.reflection_amount.clamp(0.0, 100.0),
        // bOptions2 and 6 = 4 (an interpolation hybrid of dIFS): index 1
        trans_di_const: mc.refraction_index,
        calc_trans: mc.transparency,
        only_difs: mc.only_difs,
        absorption: (mc.absorption as f64 * p.step_width) as f32,
        absorp_coeff: mc.absorption,
        light_scattering_mul: mc.scattering / 330.0,
        back_dist: (((8388352.0 / p.zc_mul + 1.0).powi(2) - 1.0) * p.step_width / p.zcorr) as f32,
        hs,
        ao,
        vol: p.vol.is_some(),
    };
    let [x0, y0, w, h] = p.rect;
    let (wu, hu) = (w as usize, h as usize);
    let threads = threads.min(hu).max(1);
    let ctx = &ctx;
    std::thread::scope(|s| {
        let mut per: Vec<Vec<(usize, &mut [u8])>> = (0..threads).map(|_| Vec::new()).collect();
        for (y, row) in rgb.chunks_mut(wu * 3).enumerate() {
            per[y % threads].push((y, row));
        }
        for list in per {
            s.spawn(move || {
                for (ry, row) in list {
                    let y = ry as i32 + y0;
                    let seed = (0x24563487i64 + (y as i64 + 1) * 0x324594A1i64) as i32;
                    let mut t = SrTracer {
                        c: ctx,
                        m: Marcher::new(p, seed),
                        sr_si: SiLight::default(),
                        normal: [0.0; 3],
                        t_amb: [0.0; 3],
                        plv: Plv {
                            view: [0.0, 0.0, 1.0],
                            abs_view: [0.0, 0.0, 1.0],
                            cam_pos: [0.0; 3],
                            obj_pos: [0.0; 3],
                            z_pos: 0.0,
                            zpos_dyn_fog: 0.0,
                            x_pos: 0.0,
                            y_pos: 0.0,
                            dep_c: [0.0; 3],
                        },
                        light_z: lv.lights.iter().map(|l| (l.pos_z, l.pos_lp)).collect(),
                        calc_trans_r: false,
                        trans_flip_inside: p.inside_rendering,
                        scale_amb_diff_down: false,
                        rnd: seed,
                        guard: 0,
                        dbg: false,
                    };
                    let dp = debug_pixel();
                    for x in 0..wu {
                        t.dbg = dp == Some((x as i32 + x0, y));
                        if let Some(c) = t.pixel(x as i32 + x0, y, &gbuf[ry * wu + x]) {
                            row[x * 3..x * 3 + 3].copy_from_slice(&c);
                        }
                    }
                }
            });
        }
    });
}

/// The per pixel state of the reflection pass on the graphics card.
#[cfg(feature = "gpu")]
#[derive(Clone)]
struct RPix {
    x: i32,
    y: i32,
    /// the G-buffer value of the pixel
    si0: SiLight,
    active: bool,
    t_amb: S3,
    t_absorb: S3,
    sd: (S3, f32, S3),
    zz: f64,
    dir: Vec3,
    pos: Vec3,
    normal: Vec3,
    plv: Plv,
}

/// `reflections` with the rays on the graphics card: every bounce of all
/// pixels is one batch (`gpu::reflect_batch`), which marches the rays and
/// calculates the normals, colours, shadows and ambient occlusion of the
/// hits; the shading and the blending of the bounces stay here, as in
/// `calc_ray` without transparency.
#[cfg(feature = "gpu")]
fn reflections_gpu(sc: &Scene, p: &CalcParams, gbuf: &[SiLight], rgb: &mut [u8], cam: &PaintCamera, lv: &LightVals, threads: usize) -> Result<(), String> {
    use crate::gpu::{ReflectLight, ReflectOpts, ReflectTask};
    let mc = &sc.mc;
    let mid = sc.stereo_mid();
    let m1 = normalise_matrix_to(1.0, &p.vgrads);
    let sw = p.step_width;
    let hs_vecs: Vec<Vec3> = lv
        .lights
        .iter()
        .map(|l| if l.positional { add(s3d(l.ln), mid) } else { rotate_vector_reverse(&norm_to(-p.step_width, s3d(l.ln)), &m1) })
        .collect();
    let hs = sc.shadows.map(|h| {
        let lights: Vec<(usize, usize)> = lv.lights.iter().enumerate().filter(|(_, l)| (h.lights >> l.idx) & 1 != 0).map(|(i, l)| (i, l.idx)).collect();
        (lights, h.soft, h.soft_radius.max(0.001), h.max_len_mul)
    });
    let ao = sc.ao.as_ref().map(|_| {
        let d = sc.deao.unwrap_or_default();
        (d.max_len, d.first_step_random)
    });
    let ctx = SrCtx {
        p,
        lv,
        cam,
        mid,
        hs_vecs: hs_vecs.clone(),
        max_reflections: mc.reflection_depth as i32,
        sr_light_amount: mc.reflection_amount.clamp(0.0, 100.0),
        trans_di_const: mc.refraction_index,
        calc_trans: false,
        only_difs: mc.only_difs,
        absorption: (mc.absorption as f64 * p.step_width) as f32,
        absorp_coeff: mc.absorption,
        light_scattering_mul: mc.scattering / 330.0,
        back_dist: (((8388352.0 / p.zc_mul + 1.0).powi(2) - 1.0) * p.step_width / p.zcorr) as f32,
        hs: hs.clone(),
        ao,
        vol: false,
    };
    // the lights of the shadow rays (calc_hs): soft shadows use the last one
    let mut lights: Vec<ReflectLight> = Vec::new();
    let (mut soft_radius, mut hs_mul) = (0.0f32, 1.0f32);
    if let Some((ls, soft, radius, mul)) = &hs {
        hs_mul = *mul;
        let sel: Vec<&(usize, usize)> = if *soft { ls.last().into_iter().collect() } else { ls.iter().collect() };
        if *soft && !sel.is_empty() {
            soft_radius = *radius;
        }
        for &(li, idx) in sel {
            let l = &lv.lights[li];
            lights.push(ReflectLight { idx: idx as u32, vec: s3(&hs_vecs[li]), positional: l.positional, lmax_l: l.lmax_l, visible: l.visible as u32 });
        }
    }
    let far_fog = lv.far_fog || lv.sqr;
    let [x0, y0, w, h] = p.rect;
    let (wu, hu) = (w as usize, h as usize);
    let n = wu * hu;
    let threads = threads.clamp(1, hu);
    // a tracer per thread for the shading (its marcher is not used)
    let new_tracer = |seed: i32| SrTracer {
        c: &ctx,
        m: Marcher::new(p, seed),
        sr_si: SiLight::default(),
        normal: [0.0; 3],
        t_amb: [0.0; 3],
        plv: Plv { view: [0.0, 0.0, 1.0], abs_view: [0.0, 0.0, 1.0], cam_pos: [0.0; 3], obj_pos: [0.0; 3], z_pos: 0.0, zpos_dyn_fog: 0.0, x_pos: 0.0, y_pos: 0.0, dep_c: [0.0; 3] },
        light_z: lv.lights.iter().map(|l| (l.pos_z, l.pos_lp)).collect(),
        calc_trans_r: false,
        trans_flip_inside: false,
        scale_amb_diff_down: false,
        rnd: seed,
        guard: 0,
        dbg: false,
    };
    let dbg_i = debug_pixel().map(|(x, y)| ((y - y0) * w + (x - x0)) as usize);
    // the surfaces of the object pixels
    let mut px: Vec<Option<RPix>> = vec![None; n];
    let mut tasks: Vec<ReflectTask> = Vec::new();
    for (i, si) in gbuf.iter().enumerate() {
        if si.zpos() < 32768 && si.si_gradient < 32768 {
            tasks.push(ReflectTask { zz: (si.zpos_fine >> 8) as f32, pixel: i as u32, ..Default::default() });
        }
    }
    let timing = std::env::var_os("MB3D_GPU_TIMING").is_some();
    let t0 = std::time::Instant::now();
    let hits = crate::gpu::reflect_batch(p, &tasks, &ReflectOpts { surface: true, hs_mul, ..Default::default() })?;
    if timing {
        eprintln!("  reflections: {} surfaces in {:.2} s", tasks.len(), t0.elapsed().as_secs_f64());
    }
    let chunk = |n: usize| n.div_ceil(threads).max(1);
    let pairs: Vec<(usize, crate::gpu::ReflectHit)> = tasks.iter().zip(&hits).map(|(t, h)| (t.pixel as usize, *h)).collect();
    std::thread::scope(|s| {
        let mut rest: &mut [Option<RPix>] = &mut px;
        let mut start = 0usize;
        // the pixels in index order: split the state by pixel ranges
        let per = chunk(n);
        while start < n {
            let (part, r) = rest.split_at_mut(per.min(rest.len()));
            rest = r;
            let lo = start;
            let hi = start + part.len();
            let mine: Vec<(usize, crate::gpu::ReflectHit)> = pairs.iter().filter(|(i, _)| *i >= lo && *i < hi).copied().collect();
            s.spawn(move || {
                let mut t = new_tracer(1);
                for (i, hit) in mine {
                    if hit.skip {
                        continue;
                    }
                    let si = gbuf[i];
                    let (x, y) = (x0 + (i % wu) as i32, y0 + (i / wu) as i32);
                    let (start, vfov) = crate::calc::pixel_ray(p, x, y);
                    t.plv.cam_pos = s3(&sub(start, mid));
                    let av = s3(&normalize(vfov));
                    t.plv.abs_view = av;
                    t.plv.view = cam.to_view(av);
                    t.plv.y_pos = y as f32 / p.height as f32;
                    t.plv.x_pos = (x + 1) as f32 / p.width as f32;
                    let ys = t.plv.y_pos;
                    let sd = match lv.depth_func {
                        1 => ys * ys,
                        0 => ys,
                        _ => ys.max(0.0).sqrt(),
                    };
                    t.plv.dep_c = lin2(lv.depth_col2, lv.depth_col, sd);
                    t.calc_light_z_pos();
                    let pos = [hit.pos[0] as f64, hit.pos[1] as f64, hit.pos[2] as f64];
                    let zz = hit.zz as f64;
                    t.plv.z_pos = (zz * sw + p.zz_stmit_dif) as f32;
                    t.plv.obj_pos = s3(&sub(pos, mid));
                    t.plv.zpos_dyn_fog = t.plv.z_pos;
                    t.scale_amb_diff_down = false;
                    t.sr_si = si;
                    let sh = t.shade(&si, None);
                    if dbg_i == Some(i) {
                        eprintln!("GPU surface: pos {pos:?} zz {zz:.4} si {si:?}\n  plv {:?}\n  light {:?} result {:?} spe {:?} {} dif {:?}", t.plv, sh.light, sh.result, sh.spe, sh.spe_a, sh.dif);
                    }
                    part[i - lo] = Some(RPix {
                        x,
                        y,
                        si0: si,
                        active: true,
                        t_amb: sh.light.map(|v| (v / 255.0).clamp(0.0, 8.0)),
                        t_absorb: sc3(sh.result, ctx.sr_light_amount),
                        sd: (sh.spe, sh.spe_a, sh.dif),
                        zz,
                        dir: vfov,
                        pos,
                        normal: normal_of(&si),
                        plv: t.plv,
                    });
                }
            });
            start = hi;
        }
    });
    // the bounces
    for rit in 1..=ctx.max_reflections {
        let mut tasks: Vec<ReflectTask> = Vec::new();
        let mut dirs: Vec<(usize, Vec3)> = Vec::new();
        for (i, q) in px.iter_mut().enumerate() {
            let Some(q) = q.as_mut().filter(|q| q.active) else { continue };
            if luma(q.t_absorb.map(f32::abs)) < 1e-4 {
                q.active = false;
                continue;
            }
            let nr = rotate_vector_reverse(&q.normal, &p.vgrads);
            let new_vec = sub(q.dir, scale(nr, 2.0 * dot(&nr, &q.dir) / sqr_len(&nr)));
            tasks.push(ReflectTask { origin: s3(&q.pos), dir: s3(&new_vec), zz: q.zz as f32, pixel: i as u32 });
            dirs.push((i, new_vec));
        }
        if tasks.is_empty() {
            break;
        }
        let q = match rit {
            1 => 2,
            2 => 1,
            _ => 0,
        };
        let opts = ReflectOpts { surface: false, lights: lights.clone(), soft_radius, hs_mul, ao: ao.map(|(m, f)| (q, m, f)), far_fog };
        let t0 = std::time::Instant::now();
        let hits = crate::gpu::reflect_batch(p, &tasks, &opts)?;
        if timing {
            eprintln!("  reflections: bounce {rit}: {} rays, {} hits, in {:.2} s", tasks.len(), hits.iter().filter(|h| h.hit).count(), t0.elapsed().as_secs_f64());
        }
        let last = rit >= ctx.max_reflections;
        let items: Vec<(usize, Vec3, crate::gpu::ReflectHit)> = dirs.iter().zip(&hits).map(|((i, d), h)| (*i, *d, *h)).collect();
        std::thread::scope(|s| {
            let mut rest: &mut [Option<RPix>] = &mut px;
            let mut start = 0usize;
            let per = chunk(n);
            while start < n {
                let (part, r) = rest.split_at_mut(per.min(rest.len()));
                rest = r;
                let lo = start;
                let hi = start + part.len();
                let mine: Vec<(usize, Vec3, crate::gpu::ReflectHit)> = items.iter().filter(|(i, _, _)| *i >= lo && *i < hi).copied().collect();
                s.spawn(move || {
                    let mut t = new_tracer(1);
                    for (i, new_vec, hit) in mine {
                        let Some(q) = part[i - lo].as_mut() else { continue };
                        t.plv = q.plv;
                        t.sr_si = q.si0;
                        // the ray starts a first step from the surface (calc_ray)
                        let step0 = (p.de_stop as f64 * 0.25).min(1.0);
                        t.plv.cam_pos = s3(&sub(add(q.pos, scale(new_vec, step0)), mid));
                        t.set_ray_paint(new_vec, hit.zz2 as f64);
                        if dbg_i == Some(i) {
                            eprintln!("GPU bounce {rit}: new_vec {new_vec:?} open_air {} zz {:.4} zz2 {:.4} de_limited {} steps {}", !hit.hit, hit.zz, hit.zz2, hit.de_limited, hit.step_count);
                        }
                        if hit.hit {
                            let mut si = hit.si;
                            if ao.is_some() {
                                si.amb_shadow = ((si.amb_shadow as i32 * 8) / (8 + rit)) as u16;
                            }
                            q.normal = normal_of(&si);
                            q.pos = [hit.pos[0] as f64, hit.pos[1] as f64, hit.pos[2] as f64];
                            q.zz = hit.zz as f64;
                            t.plv.z_pos = (q.zz.abs() * sw + p.zz_stmit_dif) as f32;
                            t.plv.obj_pos = s3(&sub(q.pos, mid));
                            t.scale_amb_diff_down = false;
                            q.t_absorb = mul3(q.t_absorb, q.sd.0);
                            let sh = t.shade(&si, None);
                            if dbg_i == Some(i) {
                                eprintln!("  hit: pos {:?} si {si:?}\n  plv {:?}\n  light {:?} result {:?} spe {:?} {} dif {:?} t_absorb {:?}", q.pos, t.plv, sh.light, sh.result, sh.spe, sh.spe_a, sh.dif, q.t_absorb);
                            }
                            let tmp_amb = sh.light.map(|v| (v / 255.0).clamp(0.0, 8.0));
                            q.sd = (sh.spe, sh.spe_a, sh.dif);
                            q.t_amb = add3(q.t_amb, mul3(q.t_absorb, tmp_amb));
                            q.t_absorb = mul3(q.t_absorb, sh.result);
                            if last {
                                q.active = false;
                            } else {
                                q.t_absorb = sc3(q.t_absorb, ctx.sr_light_amount);
                                q.dir = new_vec;
                            }
                        } else {
                            q.t_absorb = mul3(q.t_absorb, q.sd.0);
                            // the step count of the ray (dynamic fog) as calc_ray keeps it
                            let mut si = SiLight { zpos_fine: 32768 << 16, shadow: hit.step_count.clamp(0.0, 1023.0).round_ties_even() as u16, ..SiLight::default() };
                            t.t_amb = q.t_amb;
                            let mut sd = q.sd;
                            let mut ta = q.t_absorb;
                            t.calc_open_air(&mut si, &mut sd, &mut ta, false, (hit.zz2 as f64).min(p.zend) as f32);
                            q.t_amb = t.t_amb;
                            if dbg_i == Some(i) {
                                eprintln!("  open air: t_absorb {ta:?} t_amb {:?} plv {:?}", t.t_amb, t.plv);
                            }
                            q.active = false;
                        }
                        q.plv = t.plv;
                    }
                });
                start = hi;
            }
        });
    }
    // the colours
    for (i, q) in px.iter().enumerate() {
        let Some(q) = q else { continue };
        let mut t = q.t_amb.map(|v| v.clamp(0.0, 1.0));
        if lv.sqr {
            t = t.map(|v| v.sqrt());
        }
        if lv.gamma_h != 0 {
            let g = if lv.gamma_h > 0 { t.map(|v| v.sqrt()) } else { t.map(|v| v * v) };
            t = add3(t, sc3(sub3(g, t), lv.s_gamma));
        }
        let c = t.map(|v| (v * 255.0).round().clamp(0.0, 255.0) as u8);
        rgb[i * 3..i * 3 + 3].copy_from_slice(&c);
        let _ = (q.x, q.y);
    }
    Ok(())
}
