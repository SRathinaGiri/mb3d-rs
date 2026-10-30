//! Lighting parameters and the per-pixel colouring, ported from
//! `MakeLightValsFromHeaderLight` (HeaderTrafos.pas), `SetCosTabFunction` /
//! `GetCosTabVal` (LightAdjust.pas) and `CalcPixelColor2` (PaintThread.pas).
//!
//! Global and positional lights (diffuse + specular, hard / soft shadow
//! bits), visible light sources, ambient top/bottom with ambient occlusion,
//! depth fog, dynamic fog and volumetric light, the 10-colour surface palette
//! and the 4 interior colours, gamma.  Not yet ported: light maps, image
//! maps and background images.

use crate::gbuffer::SiLight;
use crate::math::{fast_int_pow, make_spline_coeff};
use std::sync::OnceLock;

/// A colour channel of a painted image: 8 bit (the pictures as MB3D
/// paints them) or 16 bit (`mb3d --bits 16`, "PNG 16 bit colour").  The
/// colours are calculated in floating point; each stage rounds its result
/// in its own way (as MB3D does for 8 bit), so the 8 bit images stay
/// exactly as before.
pub trait Sample: Copy + Default + Send + Sync + 'static {
    /// a painted colour (0 to 255 for 8 bit: `CalcPixelColor2`)
    fn from_paint(v: f32) -> Self;
    /// a colour of the reflection pass (0 to 1)
    fn from_unit(t: f32) -> Self;
    /// a colour of the depth of field (0 to 255 for 8 bit)
    fn from_dof(v: f32) -> Self;
    /// the value on the 8 bit scale (0 to 255)
    fn to_255(self) -> f32;
}

impl Sample for u8 {
    #[inline]
    fn from_paint(v: f32) -> u8 {
        v.round() as u8
    }
    #[inline]
    fn from_unit(t: f32) -> u8 {
        (t * 255.0).round().clamp(0.0, 255.0) as u8
    }
    #[inline]
    fn from_dof(v: f32) -> u8 {
        v.clamp(0.0, 255.0).round_ties_even() as u8
    }
    #[inline]
    fn to_255(self) -> f32 {
        self as f32
    }
}

impl Sample for u16 {
    #[inline]
    fn from_paint(v: f32) -> u16 {
        (v * 257.0).round() as u16
    }
    #[inline]
    fn from_unit(t: f32) -> u16 {
        (t * 65535.0).round().clamp(0.0, 65535.0) as u16
    }
    #[inline]
    fn from_dof(v: f32) -> u16 {
        (v * 257.0).clamp(0.0, 65535.0).round_ties_even() as u16
    }
    #[inline]
    fn to_255(self) -> f32 {
        self as f32 / 257.0
    }
}

/// One light source (`TLight8`, global lights only).
#[derive(Clone, Debug)]
pub struct Light {
    pub on: bool,
    pub color: [u8; 3],
    /// Horizontal / vertical angle in radians (`LXpos`, `LYpos`).
    pub x_angle: f64,
    pub y_angle: f64,
    /// `Lamp` (amplitude)
    pub amplitude: f32,
    /// Specular exponent = 2 shl spec_func (`LFunction and 7`)
    pub spec_func: i32,
    /// Diffuse function 0..3 (`(LFunction shr 4) and 3`)
    pub diff_func: i32,
    /// Light angles relative to the object instead of the viewer (`Loption bit 6`)
    pub relative_to_object: bool,
    /// Hard shadows enabled for this light (`Loption` bit 6 clear)
    pub hs_enabled: bool,
    /// Positional light (`Loption` bit 2): `position` is used instead of the angles
    pub positional: bool,
    /// Absolute position of a positional light (`LXpos`, `LYpos`, `LZpos`)
    pub position: [f64; 3],
    /// Visible light source function (`iLightPos and 14`): 0 = invisible,
    /// 2 = "vislight 3", 4 = old style, 6 = "vislight 2", 8.. = glow
    pub visible: u8,
    /// Light map number (`Loption = 2`, `LightMapNr`): an image lighting the
    /// object from all directions; 0 = normal light
    pub map: u16,
    /// rotation bytes of a light map (first bytes of `LXpos`, `LYpos`, `LZpos`)
    pub map_rot: [u8; 3],
}

impl Default for Light {
    fn default() -> Self {
        // defaultLight8
        Light {
            on: false,
            color: [255, 255, 255],
            x_angle: 0.0,
            y_angle: 0.0,
            amplitude: 1.0,
            spec_func: 4,
            diff_func: 0,
            relative_to_object: false,
            hs_enabled: true,
            positional: false,
            position: [0.0; 3],
            visible: 0,
            map: 0,
            map_rot: [0; 3],
        }
    }
}

/// A surface colour of the palette (`TLCol8`).
#[derive(Clone, Copy, Debug)]
pub struct PaletteColor {
    pub position: u16,
    pub diffuse: [u8; 3],
    pub specular: [u8; 3],
}

/// The lighting record. The scalar values use MB3D's trackbar units so that
/// they map one-to-one to `TLightingParas9.TBpos[]` (phase 2 reads them from
/// .m3p files).
#[derive(Clone, Debug)]
pub struct Lighting {
    pub lights: [Light; 6],
    /// `AmbCol` / `AmbCol2`
    pub amb_top: [u8; 3],
    pub amb_bottom: [u8; 3],
    /// `DepthCol` (top of image) / `DepthCol2` (bottom)
    pub depth_col: [u8; 3],
    pub depth_col2: [u8; 3],
    pub dyn_fog_col: [u8; 3],
    pub dyn_fog_col2: [u8; 3],
    /// TBpos[3]: fog offset (128 = 0)
    pub fog_offset: f32,
    /// TBpos[4]: depth fog amount
    pub depth_fog: f32,
    /// TBpos[5]: diffuse colour multiplier (50 = 1.0)
    pub diffuse: f32,
    /// TBpos[6]: dynamic fog amount (53 = off)
    pub dyn_fog: f32,
    /// TBpos[7]: specular multiplier (50 = 1.0)
    pub specular: f32,
    /// TBpos[8]: ambient multiplier (90 = 1.0)
    pub ambient: f32,
    /// TBpos[9] / TBpos[10]: colour mapping start / stop
    pub color_start: f32,
    pub color_end: f32,
    /// TBpos[11] low byte: ambient shadow amplitude (53 = 1.0)
    pub amb_shadow: f32,
    /// TBpos[11] high byte + 53: indirect light reflection (53 default)
    pub ind_light: f32,
    /// Gamma bits of TBoptions (32 = off, <32 darker, >32 brighter)
    pub gamma: f32,
    /// `RoughnessFactor` 0..255
    pub roughness: f32,
    /// TBoptions bit 15: colour cycling
    pub color_cycling: bool,
    /// `VarColZpos`
    pub var_col_z: f32,
    /// TBoptions bit 17: colour on orbit trap instead of iterations
    pub color_on_otrap: bool,
    /// TBoptions bit 18: far fog
    pub far_fog: bool,
    /// TBoptions bits 30..31: background (depth) colour function
    pub depth_func: u8,
    /// TBoptions bit 16 + FineColAdj1/2: fine colour adjustment (0..60, 30 = neutral)
    pub fine_col_adj: Option<(u8, u8)>,
    /// TBoptions bits 0..6 / 7..13: interior colour start / end (TB12, TB13)
    pub interior_start: f32,
    pub interior_end: f32,
    /// `Lights[3].AdditionalByteEx / 256`
    pub diffuse_shadowing: f32,
    /// Lights (bit = light index) whose hard shadow has been calculated
    /// (`bHScalculated shr 2`); set by the renderer after the shadow pass.
    pub hs_calced: u8,
    /// One soft shadow instead of hard shadows (`bCalc1HSsoft`)
    pub hs_soft: bool,
    /// Set the diffuse function of shadowed lights to cosine (`bHScalculated` bit 1)
    pub hs_set_cos: bool,
    pub palette: [PaletteColor; 10],
    pub interior: [(u16, [u8; 3]); 4],
    /// Background picture file name (`BGbmp`), empty = depth colour
    pub bg_image: String,
    /// Background in image coordinates instead of a sphere (`TBoptions` bit 15)
    pub bg_direct: bool,
    /// Background rotation (`PicOffsetX/Y/Z`, 128 = half turn)
    pub bg_rot: [u8; 3],
    /// Background brightness: 1.04^(value - 40) (`Lights[4].AdditionalByteEx`)
    pub bg_brightness: u8,
    /// Background adds light instead of being darkened by the depth fog
    pub bg_add_light: bool,
    /// Interior colour specular amounts (alpha byte of `ICols`)
    pub interior_spec: [u8; 4],
    /// Alpha byte of the palette's specular colours (`LCols[].ColorSpe shr
    /// 24`): the transparency of the Monte Carlo renderer.  None = MB3D's
    /// default, the brightest specular component.
    pub palette_alpha: Option<[u8; 10]>,
    /// "Internal gamma 2": light calculation in squared colour space
    /// (`AdditionalOptions` bit 0, `CalcPixelColorSqr`)
    pub internal_gamma2: bool,
    /// No interpolation between the palette colours (`Lights[3].FreeByte`)
    pub no_col_ipol: bool,
    /// Ambient light and depth colours relative to the object (`TBoptions` bit 29)
    pub amb_rel_obj: bool,
    /// Dynamic fog options (`Lights[0].FreeByte`): bit 0 blend, bit 1 positive only
    pub dfog_options: u8,
    /// `iExModes` (`Lights[2].FreeByte`): non-zero = second total light mode
    pub ex_mode: u8,
    /// Diffuse colour map number (`bColorMap` + `Lights[1].AdditionalByteEx`), 0 = off
    pub diff_map: u16,
    /// Diffuse map mapping (`Lights[1].FreeByte`): 0 iterations + orbit trap,
    /// 1 on normals, 2 wrapped 3D (sine), 3 wrapped 3D
    pub diff_map_mode: u8,
    /// Diffuse map offsets / rotation (bytes in `TBpos[7]`, `TBpos[8]`)
    pub diff_map_offset: [u8; 2],
    pub diff_map_rot: u8,
    /// Diffuse map scale: 1.2^(v - 30) or 1.05^(v - 30) (`Lights[2].AdditionalByteEx`)
    pub diff_map_scale: u8,
    /// Combine the diffuse map brightness with the palette colour (`AdditionalOptions` bit 2)
    pub yc_comb: bool,
    /// A small blurred copy of the background picture is the ambient light
    /// (`AdditionalOptions` bit 5)
    pub bg_ambient: bool,
}

/// `LFunction` of the start preset's main light (spec ^64, diffuse func 3)
const START_LFUNCTION: i32 = 53;

const fn rgb(c: u32) -> [u8; 3] {
    // Delphi TColor ($BBGGRR) -> [r, g, b]
    [(c & 0xFF) as u8, ((c >> 8) & 0xFF) as u8, ((c >> 16) & 0xFF) as u8]
}

impl Default for Lighting {
    /// MB3D's start-up light preset (`StartPreset` + `SetStartPreset`).
    fn default() -> Self {
        let cols: [u32; 9] =
            [5873889, 8837614, 0x8B491D, 2988346, 12248958, 0xFFC49F, 11287584, 14579248, 7481121];
        let mut palette = [PaletteColor { position: 0, diffuse: [0; 3], specular: [0; 3] }; 10];
        for (i, p) in palette.iter_mut().enumerate().take(3) {
            p.position = (i * 10922) as u16;
            p.diffuse = rgb(cols[i * 3]);
            p.specular = rgb(cols[i * 3 + 1]);
        }
        for i in 3..10 {
            palette[i] = palette[0];
            palette[i].position = (32200 + (i - 3) * 84) as u16;
        }
        let mut interior = [(0u16, [0u8; 3]); 4];
        for (i, c) in interior.iter_mut().enumerate().take(3) {
            *c = ((i * 10922) as u16, rgb(cols[i * 3]));
        }
        interior[3] = (32700, interior[0].1);
        let mut lights: [Light; 6] = Default::default();
        lights[0] = Light {
            on: true,
            color: rgb(0xA0E8FF),
            x_angle: 1500.0 * std::f64::consts::PI / 16384.0,
            y_angle: 5200.0 * std::f64::consts::PI / 16384.0,
            amplitude: 1.0,
            spec_func: START_LFUNCTION & 7,
            diff_func: (START_LFUNCTION >> 4) & 3,
            relative_to_object: false,
            hs_enabled: true,
            ..Default::default()
        };
        lights[1] = Light {
            on: false,
            color: rgb(2307911),
            x_angle: -2822.0 * std::f64::consts::PI / 16384.0,
            y_angle: -7737.0 * std::f64::consts::PI / 16384.0,
            spec_func: 16 & 7,
            diff_func: (16 >> 4) & 3,
            ..Default::default()
        };
        Lighting {
            lights,
            amb_top: rgb(0x8B491D),
            amb_bottom: rgb(0xFFC49F),
            depth_col: rgb(0x8B491D),
            depth_col2: rgb(0xFFC49F),
            dyn_fog_col: [255, 255, 255],
            dyn_fog_col2: [255, 255, 255],
            fog_offset: 128.0,
            depth_fog: 27.0,
            diffuse: 120.0,
            dyn_fog: 53.0,
            specular: 50.0,
            ambient: 90.0,
            color_start: 68.0,
            color_end: 72.0,
            amb_shadow: 53.0,
            ind_light: 53.0,
            gamma: 32.0,
            roughness: 0.0,
            color_cycling: false,
            var_col_z: 0.0,
            color_on_otrap: false,
            far_fog: false,
            depth_func: 0,
            fine_col_adj: None,
            interior_start: 60.0,
            interior_end: 60.0,
            diffuse_shadowing: 0.0,
            hs_calced: 0,
            hs_soft: false,
            hs_set_cos: false,
            palette,
            interior,
            bg_image: String::new(),
            bg_direct: false,
            bg_rot: [0; 3],
            bg_brightness: 40,
            bg_add_light: false,
            interior_spec: [160; 4],
            palette_alpha: None,
            internal_gamma2: false,
            no_col_ipol: false,
            amb_rel_obj: false,
            dfog_options: 0,
            ex_mode: 0,
            diff_map: 0,
            diff_map_mode: 0,
            diff_map_offset: [0, 0],
            diff_map_rot: 128,
            diff_map_scale: 30,
            yc_comb: false,
            bg_ambient: false,
        }
    }
}

/// `BuildRotMatrixS` from MB3D's byte angles (128 = pi), x angle mirrored.
fn pic_rot_matrix(b: [u8; 3]) -> [[f32; 3]; 3] {
    let k = std::f64::consts::PI / 128.0;
    let (xa, ya, za) = (std::f64::consts::PI - b[0] as f64 * k, b[1] as f64 * k, b[2] as f64 * k);
    let (sx, cx) = xa.sin_cos();
    let (sy, cy) = ya.sin_cos();
    let (sz, cz) = za.sin_cos();
    let m = [
        [cy * cz, -cy * sz, sy],
        [sx * sy * cz + cx * sz, cx * cz - sx * sy * sz, -sx * cy],
        [sx * sz - cx * sy * cz, cx * sy * sz + sx * cz, cx * cy],
    ];
    m.map(|r| r.map(|v| v as f32))
}

const IDENT3F: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

/// A map used by the painter, with its rotation and intensity.
pub(crate) struct PaintMap {
    pub(crate) map: std::sync::Arc<crate::maps::LightMap>,
    pub(crate) rot: [[f32; 3]; 3],
    pub(crate) intensity: f32,
}

impl Lighting {
    /// The alpha byte of palette entry `i` (see [`Lighting::palette_alpha`]).
    pub fn palette_alpha(&self, i: usize) -> u8 {
        match self.palette_alpha {
            Some(a) => a[i],
            None => {
                let s = self.palette[i].specular;
                s[0].max(s[1]).max(s[2])
            }
        }
    }

    /// Replace the surface palette by evenly spaced colours (diffuse only,
    /// specular = a lighter tint).
    pub fn set_palette(&mut self, cols: &[[u8; 3]]) {
        if cols.is_empty() {
            return;
        }
        let n = cols.len().min(10);
        for i in 0..10 {
            let c = cols[i.min(n - 1)];
            let spec = [
                ((c[0] as u32 + 255) / 2) as u8,
                ((c[1] as u32 + 255) / 2) as u8,
                ((c[2] as u32 + 255) / 2) as u8,
            ];
            self.palette[i] = PaletteColor {
                position: if i < n { (i * 32767 / n) as u16 } else { (32200 + (i - n) * 60) as u16 },
                diffuse: if i < n { c } else { cols[0] },
                specular: spec,
            };
            if i >= n {
                self.palette[i].specular = self.palette[0].specular;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Light look-up tables (SetCosTabFunction)
// ---------------------------------------------------------------------------

struct CosTabs {
    diff: [[f32; 128]; 8],
}

fn cos_tabs() -> &'static CosTabs {
    static T: OnceLock<CosTabs> = OnceLock::new();
    T.get_or_init(|| {
        let mut diff = [[0f32; 128]; 8];
        for i in 0..128 {
            let d = 1.0 - (i as f64 - 2.0) / 60.0;
            diff[0][i] = if d > 0.15 {
                ((d - 0.08) * 1.0869565) as f32
            } else if d <= 0.0 {
                0.0
            } else {
                d.powf(f64::max(1.0, (0.505 - d) * 3.8)) as f32
            };
            let dc = d.max(0.0);
            diff[1][i] = (dc * dc) as f32;
            diff[2][i] = (d * 0.5 + 0.5) as f32;
            diff[3][i] = ((d * 0.5 + 0.5) * (d * 0.5 + 0.5)) as f32;
        }
        for k in 0..4 {
            let tmp: Vec<f32> = (0..128).map(|j| diff[k][j].max(0.0).sqrt()).collect();
            for j in 0..128 {
                let mut e = 0f64;
                for i in 0..=60i32 {
                    let l = (j as i32 + i - 30).unsigned_abs() as usize;
                    if l < 128 {
                        e += tmp[l] as f64;
                    }
                }
                let a = e * 0.011 + (e * 0.007) * (e * 0.007);
                diff[k + 4][j] = (a * a) as f32;
            }
        }
        CosTabs { diff }
    })
}

/// `GetCosTabVal`: diffuse light function with roughness blending.
pub(crate) fn get_cos_tab_val(tnr: usize, dotp: f32, rough: f32) -> f32 {
    cos_tab_val(tnr, dotp, rough, false)
}

/// `GetCosTabValSqr` (internal gamma 2, `CalcPixelColorSqr`): the two
/// interpolated values squared before the roughness blend.
pub(crate) fn get_cos_tab_val_sqr(tnr: usize, dotp: f32, rough: f32) -> f32 {
    cos_tab_val(tnr, dotp, rough, true)
}

fn cos_tab_val(tnr: usize, dotp: f32, rough: f32, sqr: bool) -> f32 {
    let tabs = cos_tabs();
    let mut t = 62.0 - 60.0 * dotp;
    let mut ip = t.trunc() as i32 - 1;
    if ip < 0 {
        ip = 0;
        t = 0.0;
    } else if ip > 124 {
        ip = 124;
        t = 1.0;
    } else {
        t = t.fract();
    }
    let w = make_spline_coeff(t as f64);
    let ip = ip as usize;
    let p1 = &tabs.diff[tnr][ip..ip + 4];
    let p2 = &tabs.diff[tnr + 4][ip..ip + 4];
    let mut a: f32 = (0..4).map(|k| p1[k] * w[k]).sum();
    let mut b: f32 = (0..4).map(|k| p2[k] * w[k]).sum();
    if sqr {
        a *= a;
        b *= b;
    }
    a + rough * (b - a)
}

// ---------------------------------------------------------------------------
// Derived light values (TLightVals)
// ---------------------------------------------------------------------------

type SVec = [f32; 3];

#[inline]
fn sv_add(a: SVec, b: SVec) -> SVec {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
#[inline]
fn sv_scale(a: SVec, s: f32) -> SVec {
    [a[0] * s, a[1] * s, a[2] * s]
}
#[inline]
fn sv_mul(a: SVec, b: SVec) -> SVec {
    [a[0] * b[0], a[1] * b[1], a[2] * b[2]]
}
#[inline]
fn sv_add_w(a: &mut SVec, b: SVec, w: f32) {
    a[0] += b[0] * w;
    a[1] += b[1] * w;
    a[2] += b[2] * w;
}
/// `LinInterpolate2SVecs(sv1, sv2, w1)` = sv1 * w1 + sv2 * (1 - w1)
#[inline]
fn lerp(sv1: SVec, sv2: SVec, w1: f32) -> SVec {
    [
        sv2[0] + w1 * (sv1[0] - sv2[0]),
        sv2[1] + w1 * (sv1[1] - sv2[1]),
        sv2[2] + w1 * (sv1[2] - sv2[2]),
    ]
}
fn col255(c: [u8; 3]) -> SVec {
    [c[0] as f32, c[1] as f32, c[2] as f32]
}
fn col1(c: [u8; 3]) -> SVec {
    sv_scale(col255(c), 1.0 / 255.0)
}

#[derive(Clone)]
pub(crate) struct LightVal {
    /// header light index 0..5
    pub(crate) idx: usize,
    /// light switched on (`iLightOption = 0`); off lights only exist while
    /// the light values of animation keyframes are blended
    pub(crate) on: bool,
    pub(crate) sub_amb_sh: bool,
    /// hard shadow calculated for this light
    pub(crate) hs_calced: bool,
    /// shadow bit ($400 shl idx), u16::MAX = soft shadow value
    pub(crate) hs_mask: u16,
    pub(crate) ln: SVec,
    pub(crate) col: SVec,
    pub(crate) pow_func: i32,
    pub(crate) diff_func: usize,
    /// positional light: `ln` is the position relative to the scene middle
    pub(crate) positional: bool,
    /// visible light function (`iLightPos and 14`)
    pub(crate) visible: u8,
    /// `sLmaxL`: maximum squared distance of a positional light
    pub(crate) lmax_l: f32,
    /// `sPosLightZpos`, `sPosLP`: depth of a visible light
    pub(crate) pos_z: f32,
    pub(crate) pos_lp: f32,
}

/// Values pre-computed from the lighting record (`TLightVals`).
pub struct LightVals {
    pub(crate) lights: Vec<LightVal>,
    pub(crate) amb_col: SVec,
    pub(crate) amb_col2: SVec,
    pub(crate) depth_col: SVec,
    pub(crate) depth_col2: SVec,
    pub(crate) dyn_fog_col: SVec,
    pub(crate) dyn_fog_col2: SVec,
    pub(crate) s_depth: f32,
    pub(crate) s_shad: f32,
    pub(crate) s_shad_gr: f32,
    pub(crate) s_shad_zmul: f32,
    pub(crate) s_dyn_fog_mul: f32,
    pub(crate) s_amb_shad: f32,
    pub(crate) s_diff: f32,
    pub(crate) s_spec: f32,
    pub(crate) s_ind_light_reflect: f32,
    pub(crate) s_col_zmul: f32,
    pub(crate) s_c_start: f32,
    pub(crate) s_c_mul: f32,
    pub(crate) s_ci_start: f32,
    pub(crate) s_ci_mul: f32,
    pub(crate) s_roughness_factor: f32,
    pub(crate) col_cycling: bool,
    pub(crate) col_on_otrap: bool,
    pub(crate) far_fog: bool,
    pub(crate) depth_func: u8,
    pub(crate) s_diffuse_shadowing: f32,
    pub(crate) gamma_h: i32,
    pub(crate) s_gamma: f32,
    pub(crate) col_dif: [SVec; 10],
    pub(crate) col_spe: [SVec; 10],
    /// alpha of the specular colours (transparency in the MC renderer)
    pub(crate) col_spe_a: [f32; 10],
    pub(crate) col_pos: [i32; 10],
    pub(crate) s_c_div: [f32; 10],
    pub(crate) col_int: [SVec; 4],
    pub(crate) icol_pos: [i32; 4],
    pub(crate) s_ic_div: [f32; 4],
    pub(crate) vol_light: bool,
    /// background picture (`bBackBMP`) and its options
    pub(crate) bg: Option<PaintMap>,
    pub(crate) bg_direct: bool,
    pub(crate) bg_add_light: bool,
    /// light map lights
    pub(crate) map_lights: Vec<PaintMap>,
    /// light map lights relative to the object (rotation combined later)
    pub(crate) map_lights_rel: Vec<bool>,
    /// header light index of each map light
    pub(crate) map_lights_idx: Vec<usize>,
    pub(crate) col_int_spec: [f32; 4],
    pub(crate) sqr: bool,
    pub(crate) no_col_ipol: bool,
    pub(crate) amb_rel_obj: bool,
    pub(crate) dfog_options: u8,
    pub(crate) ex_mode: bool,
    pub(crate) yc_comb: bool,
    pub(crate) diff_map: Option<DiffMap>,
    /// `BGsmallLM` when the background picture is the ambient light
    pub(crate) bg_small: Option<PaintMap>,
    /// `lvMidPos` (for the wrapped 3D diffuse map)
    pub(crate) mid: [f64; 3],
}

/// The diffuse colour map (`DiffColLightMap` + `iColOnOT`).
pub(crate) struct DiffMap {
    pub(crate) map: std::sync::Arc<crate::maps::LightMap>,
    /// `iColOnOT shr 1`: 1 = iterations + orbit trap, 2 = on normals,
    /// 3 = wrapped 3D (sine), 4 = wrapped 3D
    pub(crate) kind: u8,
    pub(crate) off: [f32; 2],
    pub(crate) rot_sin: f32,
    pub(crate) rot_cos: f32,
    pub(crate) scale: f32,
    pub(crate) rot: [[f32; 3]; 3],
}

/// `YofSVec`
#[inline]
fn luma(v: SVec) -> f32 {
    v[0] * 0.3 + v[1] * 0.59 + v[2] * 0.11
}

/// 3x3 matrix product a * b (`Multiply2SMatrix`).
fn mat_mul(a: &[[f32; 3]; 3], b: &[[f32; 3]; 3]) -> [[f32; 3]; 3] {
    let mut r = [[0f32; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            r[i][j] = a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j];
        }
    }
    r
}

/// Delphi `Frac`
#[inline]
fn frac(x: f64) -> f32 {
    (x - x.trunc()) as f32
}

fn hs_calced(l: &Lighting, idx: usize) -> bool {
    (l.hs_calced >> idx) & 1 != 0
}

/// Camera / z-buffer values needed by the painter.
pub struct PaintCamera {
    pub width: i32,
    pub height: i32,
    pub fov: f32,
    pub aspect: f32,
    pub x_off: f32,
    pub planar: i32,
    pub pl_optic_z: f32,
    pub zcorr: f64,
    pub zc_mul: f64,
    pub step_width: f64,
    pub zz_stmit_dif: f64,
    /// `GetStartSPosAndAddVecs`: unit view matrix (rows = image x, y, view
    /// direction), position of pixel (0, 0) on the start plane relative to
    /// the scene middle, and the per-pixel increments
    pub m: [[f32; 3]; 3],
    pub start_pos: SVec,
    pub x_add: SVec,
    pub y_add: SVec,
}

impl PaintCamera {
    /// `CalcViewVec` for image coordinates in 0..1
    pub(crate) fn view_vec(&self, x_pos: f32, y_pos: f32, aspect: f32) -> SVec {
        let cx = (self.x_off - x_pos) * self.fov * aspect;
        let cy = (y_pos - 0.5) * self.fov;
        match self.planar {
            1 => {
                let v = [-cx, cy, self.pl_optic_z];
                let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
                sv_scale(v, 1.0 / n)
            }
            2 => [-(cx.sin()) * cy.cos(), cy.sin(), cy.cos() * cx.cos()],
            _ => {
                let v = [-(cx.sin()), cy.sin(), cy.cos() * cx.cos()];
                let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
                sv_scale(v, 1.0 / n)
            }
        }
    }

    /// view space -> scene (`RotateSVectorS` with the paint matrix)
    pub(crate) fn to_abs(&self, v: SVec) -> SVec {
        let m = &self.m;
        [
            v[0] * m[0][0] + v[1] * m[1][0] + v[2] * m[2][0],
            v[0] * m[0][1] + v[1] * m[1][1] + v[2] * m[2][1],
            v[0] * m[0][2] + v[1] * m[1][2] + v[2] * m[2][2],
        ]
    }

    /// scene -> view space (`RotateSVectorReverseS`)
    pub(crate) fn to_view(&self, v: SVec) -> SVec {
        let m = &self.m;
        [sv_dot(m[0], v), sv_dot(m[1], v), sv_dot(m[2], v)]
    }

    /// Position of a calculated object pixel relative to the scene middle
    /// (`CalcObjPos`), None for background.
    pub fn object_pos(&self, si: &SiLight, x: i32, y: i32) -> Option<[f64; 3]> {
        if si.zpos() > 32767 {
            return None;
        }
        let view = self.view_vec((x + 1) as f32 / self.width as f32, y as f32 / self.height as f32, self.aspect);
        let z1 = (((8388352 - (si.zpos_fine >> 8) as i64) as f64 / self.zc_mul + 1.0).powi(2) - 1.0) * self.step_width / self.zcorr;
        let a = self.to_abs(view);
        let c = self.cam_pos(x as f32, y as f32);
        Some([c[0] as f64 + a[0] as f64 * z1, c[1] as f64 + a[1] as f64 * z1, c[2] as f64 + a[2] as f64 * z1])
    }

    /// Unit view direction of a pixel in scene space.
    pub fn pixel_dir(&self, x: f32, y: f32) -> [f64; 3] {
        let v = self.to_abs(self.view_vec((x + 1.0) / self.width as f32, y / self.height as f32, self.aspect));
        [v[0] as f64, v[1] as f64, v[2] as f64]
    }

    /// Camera position of a (fractional) pixel on the start plane.
    pub(crate) fn cam_pos(&self, x: f32, y: f32) -> SVec {
        if self.planar == 2 {
            self.start_pos
        } else {
            sv_add(sv_add(self.start_pos, sv_scale(self.y_add, y)), sv_scale(self.x_add, x))
        }
    }
}

#[inline]
fn sv_dot(a: SVec, b: SVec) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
#[inline]
fn sv_sub(a: SVec, b: SVec) -> SVec {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
/// `SqrDistSV`: squared distance of point `a` from the line along unit `b`
#[inline]
fn sqr_dist_sv(a: SVec, b: SVec) -> f32 {
    let c = [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    sv_dot(c, c)
}

/// `CalcXYZposForLight` (z only): distance of a positional light from the
/// start plane along the view ray through it.  The ray is found by a small
/// gradient search over the image coordinates.
fn calc_z_pos_for_light(cam: &PaintCamera, lpos: SVec, positional: bool, view_z: [f64; 3]) -> f32 {
    if !positional {
        return 0.0;
    }
    let (w, h) = (cam.width as f32, cam.height as f32);
    let aspect = w / h;
    let plv = |mx: f32, my: f32| -> (SVec, SVec) {
        let sp = sv_add(sv_add(cam.start_pos, sv_scale(cam.y_add, my)), sv_scale(cam.x_add, mx));
        let v = cam.view_vec(mx / w, my / h, aspect);
        (sp, cam.to_abs(v))
    };
    let dist = |abs: SVec, sp: SVec| -> f64 {
        // DistanceFromViewVecToLightPos
        let v1 = sv_sub(lpos, sp);
        let v2 = [
            v1[1] * (v1[2] - abs[2]) - v1[2] * (v1[1] - abs[1]),
            v1[2] * (v1[0] - abs[0]) - v1[0] * (v1[2] - abs[2]),
            v1[0] * (v1[1] - abs[1]) - v1[1] * (v1[0] - abs[0]),
        ];
        sv_dot(v2, v2).sqrt() as f64
    };
    let dmul = 0.001 / cam.step_width;
    let (mut x, mut y) = (w * 0.5, h * 0.5);
    let (mut sp, mut abs) = plv(x, y);
    for _ in 0..=20 {
        let r = plv(x, y);
        sp = r.0;
        abs = r.1;
        let d = dist(abs, sp) * dmul;
        if d < 1e-4 {
            break;
        }
        let (s1, a1) = plv(x + 0.01, y);
        let dx = d - dist(a1, s1) * dmul;
        let (s2, a2) = plv(x, y + 0.01);
        let dy = d - dist(a2, s2) * dmul;
        let dd = (dx * dx + dy * dy).sqrt() + 1e-16;
        // the last evaluated ray is the one used for z below
        sp = s2;
        abs = a2;
        if dd * 2.0 > d {
            break;
        }
        x += (dx.signum() * d * 4e-3 / dd) as f32;
        y += (dy.signum() * d * 4e-3 / dd) as f32;
    }
    let mut z = if abs[0].abs() > 0.5 {
        (lpos[0] - sp[0]) / abs[0]
    } else if abs[1].abs() > 0.5 {
        (lpos[1] - sp[1]) / abs[1]
    } else {
        (lpos[2] - sp[2]) / abs[2]
    } as f64;
    // distance of the light from the view plane
    let dz = cam.zz_stmit_dif;
    let dv = [lpos[0] as f64 - view_z[0] * dz, lpos[1] as f64 - view_z[1] * dz, lpos[2] as f64 - view_z[2] * dz];
    let d = dv[0] * view_z[0] + dv[1] * view_z[1] + dv[2] * view_z[2];
    if d.abs() > z.abs() {
        z = d;
    }
    z.max(0.0) as f32
}

/// `CalcPosLightShape`: brightness (`flux`) and transparency of a visible
/// light source from the squared distance of the view ray to the light.
pub(crate) fn pos_light_shape(flux: &mut f32, transp: &mut f32, func: u8, pos: bool, sqr: bool) {
    let tmp_r = *transp;
    let rtmp_r = 1.0 / tmp_r;
    match func {
        4 => {
            let t = *flux * rtmp_r;
            *transp = (t * t) * (t * t);
            *flux = 7e-3 * (tmp_r - *flux) * rtmp_r * rtmp_r;
        }
        2 => {
            *transp = 1.0;
            let f = 1.0 - (*flux * rtmp_r).sqrt();
            *flux = 5e-2 * f.max(0.0).powi(50) * rtmp_r;
            if pos {
                *flux *= 1.5;
            }
        }
        6 => {
            let f = *flux * 1.05 * rtmp_r;
            if f < 1.0 {
                *transp = if pos { (f - 0.95).max(0.0) * 10.0 } else { 0.7 };
                *flux = 4e-3 * (1.05 - f * f).sqrt() * rtmp_r;
                if !pos {
                    *flux *= 0.7;
                }
            } else if pos {
                *transp = (f - 0.95) * 10.0;
                *flux = 17.8e-3 * (1.05 - f) * rtmp_r;
            } else {
                *transp = (f - 1.0) * 6.0 + 0.7;
                *flux = 12.5e-3 * (1.05 - f) * rtmp_r;
            }
        }
        _ => {
            *transp = 1.0;
            let t = *flux * rtmp_r;
            let k = if sqr { 40.0 } else { 25.0 };
            *flux = (1.0 - t) * 0.1 / ((t + 0.0004) * k).powi(2) * rtmp_r;
        }
    }
}

/// `ConvertVLight`: decodes the volumetric light amount (7 bit mantissa,
/// 3 bit exponent) stored in the shadow word.
#[inline]
pub fn convert_vlight(w: u16) -> u32 {
    let w = (w & 0x3FF) as u32;
    (w & 0x7F) << ((w >> 7) & 7)
}

/// `RMcalcVLight`: encodes a volumetric light amount for the shadow word.
pub fn encode_vlight(step_count: f32) -> u16 {
    let mut i = (step_count.round_ties_even() as i64).clamp(i32::MIN as i64, 16383) as i32;
    if i <= 0 {
        return (i & 0x3FF) as u16;
    }
    let bits = 31 - i.leading_zeros() as i32; // bsr
    let sh = bits - 6;
    if sh > 0 {
        i = (i >> sh) | (sh << 7);
    }
    i as u16
}

impl LightVals {
    /// `MakeLightValsFromHeaderLight` (subset).
    pub fn new(l: &Lighting, z_step_div: f64, dfog_on_it: u16, z_range: f64, step_width: f64, width: i32, image_scale: f64) -> LightVals {
        Self::build(l, z_step_div, dfog_on_it, z_range, step_width, width, image_scale, false)
    }

    /// Like [`LightVals::new`], but with the lights that are switched off too
    /// (with colour 0, as MB3D clears `sLCols` of off lights before it
    /// interpolates); see [`LightVals::blend`].
    #[allow(clippy::too_many_arguments)]
    pub fn new_all(l: &Lighting, z_step_div: f64, dfog_on_it: u16, z_range: f64, step_width: f64, width: i32, image_scale: f64) -> LightVals {
        Self::build(l, z_step_div, dfog_on_it, z_range, step_width, width, image_scale, true)
    }

    #[allow(clippy::too_many_arguments)]
    fn build(l: &Lighting, z_step_div: f64, dfog_on_it: u16, z_range: f64, step_width: f64, width: i32, image_scale: f64, all: bool) -> LightVals {
        let sqr = l.internal_gamma2;
        let light_scale: f32 = if sqr { 1.5 } else { 1.0 };
        // RGBColToSVecNoScale(SQR)
        let lcol = |c: [u8; 3]| -> SVec {
            if sqr {
                c.map(|v| v as f32 * v as f32 / 255.0)
            } else {
                col255(c)
            }
        };
        // ColToSVec
        let pcol = |c: [u8; 3]| -> SVec {
            if sqr {
                c.map(|v| v as f32 * v as f32 * 0.0000153787)
            } else {
                col1(c)
            }
        };
        let mut lights = Vec::new();
        for (idx, li) in l.lights.iter().enumerate().filter(|(_, li)| all || li.on) {
            // BuildViewVectorFOV(LY, -LX), negated
            let dtmp = -li.x_angle;
            let dtmp2 = li.y_angle;
            let v = [-(dtmp.sin()), dtmp2.sin(), dtmp2.cos() * dtmp.cos()];
            let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            let ln = [(-v[0] / n) as f32, (-v[1] / n) as f32, (-v[2] / n) as f32];
            let raw = sv_scale(lcol(li.color), light_scale);
            let (col, mut lmax_l) = if li.positional {
                let c = sv_scale(raw, li.amplitude * 1.3 * light_scale);
                // sLmaxL := 800 * (HSumSVec(sLCols) + 128 * sTmp), and
                // HSumSVec is (sv[0] + sv[1] + sv[3]) / 3 with sv[3] = 0
                // (MB3D's own slip, but it decides where a positional light
                // ends: with the sum of r, g and b the light reached ~3x
                // as far, 180430's light 6 lit what MB3D leaves dark)
                (c, 800.0 * ((c[0] + c[1]) / 3.0 + 128.0 * li.amplitude))
            } else {
                (sv_scale(raw, li.amplitude), 800.0 * (raw[0] + raw[1] + raw[2] + 128.0) * li.amplitude)
            };
            if li.visible == 8 {
                lmax_l *= 5.0;
            }
            lights.push(LightVal {
                on: li.on,
                ln,
                positional: li.positional,
                visible: li.visible,
                lmax_l,
                pos_z: 0.0,
                pos_lp: 0.0,
                col: if li.on { col } else { [0.0; 3] },
                pow_func: 2 << (li.spec_func & 7),
                diff_func: if hs_calced(l, idx) && l.hs_set_cos { 0 } else { (li.diff_func & 3) as usize },
                idx,
                // bSubAmbSh := iHScalced xor iHSenabled
                sub_amb_sh: hs_calced(l, idx) != li.hs_enabled,
                hs_calced: hs_calced(l, idx),
                hs_mask: if l.hs_soft && hs_calced(l, idx) { u16::MAX } else { 0x400 << idx },
            });
        }
        // relative_to_object lights are rotated in `Renderer::new` (needs the matrix)
        let s_depth = l.depth_fog * 0.8e-6;
        let (s_shad_gr, dtmp, s_dyn_fog_mul, stmp) = {
            let mut gr = (l.dyn_fog - 53.0) * image_scale as f32 * z_step_div as f32 * 0.00065;
            let mut dtmp = 2.2 / z_step_div as f32;
            let mut mul = z_step_div as f32 * 0.015;
            let mut stmp = 128.0;
            if dfog_on_it > 0 {
                dtmp *= 0.25;
                gr *= 4.0;
                mul *= 4.0;
            } else {
                stmp = 137.0;
            }
            (gr, dtmp, mul, stmp)
        };
        let s_shad = (stmp - l.fog_offset.max(0.0).sqrt() * 11.313708) * dtmp * 0.28;
        let s_shad_zmul = dtmp * 0.7 / z_range as f32 * (128.0 - 128f32.sqrt() * 11.313708);
        let s_amb_shad = l.amb_shadow / 53.0;
        // CalcSCstartAndSCmul
        let s_c_start = ((l.color_start + 30.0) / 90.0).powi(2) * 32767.0 - 10900.0;
        let mut s_c_mul = ((l.color_end + 30.0) / 90.0).powi(2) * 32767.0 - 10900.0 - s_c_start;
        let mut s_c_start = s_c_start;
        if let Some((a1, a2)) = l.fine_col_adj {
            let d = s_c_start + s_c_mul * (a2 as f32 - 30.0) * 0.0166666666666666;
            s_c_start += s_c_mul * (a1 as f32 - 30.0) * 0.0166666666666666;
            s_c_mul = d - s_c_start;
        }
        s_c_mul = if s_c_mul.abs() > 0.001 {
            2.0 / s_c_mul
        } else if s_c_mul < 0.0 {
            -2000.0
        } else {
            2000.0
        };
        // interior: TBoptions bits 0..6 / 7..13 (TB12/TB13 = 60)
        let (ti1, ti2) = (l.interior_start, l.interior_end);
        let dt = ti1.powi(4);
        let s_ci_start = dt * 0.000158025 + 32768.0;
        let mut s_ci_mul = (ti2.powi(4) - dt) * 0.5 * 0.00031605;
        if s_ci_mul.abs() > 1e-10 {
            s_ci_mul = 1.0 / s_ci_mul;
        }
        let gi = l.gamma.round() as i32;
        let (gamma_h, s_gamma) = if gi == 32 {
            (0, 0.0)
        } else if gi < 32 {
            (-1, 1.0 - gi as f32 / 32.0)
        } else {
            (1, (gi - 32) as f32 / 31.0)
        };
        let mut amb_mul = l.ambient / 90.0;
        if sqr {
            amb_mul = (amb_mul * amb_mul + amb_mul) * 0.5 * light_scale;
        }
        let mut col_dif = [[0f32; 3]; 10];
        let mut col_spe = [[0f32; 3]; 10];
        let mut col_spe_a = [0f32; 10];
        let mut col_pos = [0i32; 10];
        for i in 0..10 {
            col_dif[i] = pcol(l.palette[i].diffuse);
            col_spe[i] = pcol(l.palette[i].specular);
            let a = l.palette_alpha(i) as f32;
            col_spe_a[i] = if sqr { a * a * 0.0000153787 } else { a / 255.0 };
            col_pos[i] = l.palette[i].position as i32;
        }
        let mut s_c_div = [1f32; 10];
        for i in 0..10 {
            let j = if i < 9 { col_pos[i + 1] - col_pos[i] } else { 32767 - col_pos[9] };
            s_c_div[i] = if j > 1 { 1.0 / j as f32 } else { 1.0 };
        }
        let mut col_int = [[0f32; 3]; 4];
        let mut icol_pos = [0i32; 4];
        for i in 0..4 {
            col_int[i] = pcol(l.interior[i].1);
            icol_pos[i] = l.interior[i].0 as i32;
        }
        let mut s_ic_div = [1f32; 4];
        for i in 0..4 {
            let j = if i < 3 { icol_pos[i + 1] - icol_pos[i] } else { 32767 - icol_pos[3] };
            s_ic_div[i] = if j > 1 { 1.0 / j as f32 } else { 1.0 };
        }
        let ind = (l.ind_light - 53.0).clamp(-128.0, 127.0).round();
        let mut s_ind_light_reflect = ((ind + 53.0) * 0.022).powi(2);
        let mut s_diff = l.diffuse * 0.02;
        let mut s_spec = (l.specular * 0.02).max(0.004);
        if sqr {
            s_ind_light_reflect *= 0.5;
            s_diff = (s_diff * s_diff + s_diff) * 0.5;
            s_spec = (s_spec * s_spec + s_spec) * 0.5;
        }
        let col_int_spec = l.interior_spec.map(|a| if sqr { a as f32 * a as f32 * 0.0000153787 } else { a as f32 / 255.0 });
        let diff_map = if l.diff_map == 0 {
            None
        } else {
            crate::maps::by_number(l.diff_map as i32).map(|map| {
                let kind = (l.diff_map_mode & 3) + 1;
                let [ox, oy] = l.diff_map_offset;
                let (rot_sin, rot_cos, scale, rot) = if kind > 1 {
                    (0.0, 1.0, 1.2f32.powi(l.diff_map_scale as i32 - 30), pic_rot_matrix([ox, oy, l.diff_map_rot]))
                } else {
                    let a = (l.diff_map_rot as f64 - 128.0) * std::f64::consts::TAU / 256.0;
                    (a.sin() as f32, a.cos() as f32, 1.05f32.powi(l.diff_map_scale as i32 - 30), IDENT3F)
                };
                DiffMap {
                    map,
                    kind,
                    off: [(ox as f32 + 256.0) / 256.0, (oy as f32 + 256.0) / 256.0],
                    rot_sin,
                    rot_cos,
                    scale,
                    rot,
                }
            })
        };
        LightVals {
            lights,
            amb_col: sv_scale(lcol(l.amb_top), amb_mul),
            amb_col2: sv_scale(lcol(l.amb_bottom), amb_mul),
            depth_col: lcol(l.depth_col),
            depth_col2: lcol(l.depth_col2),
            dyn_fog_col: if sqr { l.dyn_fog_col.map(|v| v as f32 * v as f32 / 255.0) } else { col255(l.dyn_fog_col) },
            dyn_fog_col2: lcol(l.dyn_fog_col2),
            s_depth,
            s_shad,
            s_shad_gr,
            s_shad_zmul,
            s_dyn_fog_mul,
            s_amb_shad,
            s_diff,
            s_spec,
            s_ind_light_reflect,
            s_col_zmul: (l.var_col_z as f64 * -0.005 / (step_width * width as f64)) as f32,
            s_c_start,
            s_c_mul,
            s_ci_start,
            s_ci_mul,
            s_roughness_factor: l.roughness / (255.0 * 255.0),
            col_cycling: l.color_cycling,
            col_on_otrap: l.color_on_otrap,
            far_fog: l.far_fog,
            depth_func: l.depth_func,
            s_diffuse_shadowing: l.diffuse_shadowing,
            gamma_h,
            s_gamma,
            col_dif,
            col_spe,
            col_spe_a,
            col_pos,
            s_c_div,
            col_int,
            icol_pos,
            s_ic_div,
            vol_light: false,
            bg: if l.bg_image.is_empty() {
                None
            } else {
                crate::maps::by_name(&l.bg_image).map(|map| PaintMap {
                    map,
                    rot: pic_rot_matrix(l.bg_rot),
                    intensity: 1.04f32.powi(l.bg_brightness as i32 - 40),
                })
            },
            bg_direct: l.bg_direct,
            bg_add_light: l.bg_add_light,
            map_lights: l
                .lights
                .iter()
                .filter(|li| li.map > 0)
                .filter_map(|li| {
                    crate::maps::by_number(li.map as i32).map(|map| PaintMap {
                        map,
                        rot: pic_rot_matrix(li.map_rot),
                        intensity: li.amplitude,
                    })
                })
                .collect(),
            map_lights_rel: l
                .lights
                .iter()
                .filter(|li| li.map > 0 && crate::maps::by_number(li.map as i32).is_some())
                .map(|li| li.relative_to_object)
                .collect(),
            map_lights_idx: (0..6)
                .filter(|&i| l.lights[i].map > 0 && crate::maps::by_number(l.lights[i].map as i32).is_some())
                .collect(),
            col_int_spec,
            sqr,
            no_col_ipol: l.no_col_ipol,
            amb_rel_obj: l.amb_rel_obj,
            dfog_options: if l.dfog_options & 3 == 3 { 1 } else { l.dfog_options & 3 },
            ex_mode: l.ex_mode != 0,
            yc_comb: l.yc_comb,
            diff_map,
            bg_small: if l.bg_ambient && !l.bg_image.is_empty() {
                crate::maps::by_name(&l.bg_image).map(|map| PaintMap {
                    map: std::sync::Arc::new(map.small_copy()),
                    rot: pic_rot_matrix(l.bg_rot),
                    intensity: 1.04f32.powi(l.bg_brightness as i32 - 40),
                })
            } else {
                None
            },
            mid: [0.0; 3],
        }
    }

    /// Volumetric light: the shadow word holds the light amount
    /// (`bVolLight` in `MakeLightValsFromHeaderLight`).
    pub fn set_vol_light(&mut self, l: &Lighting, z_range: f64) {
        self.vol_light = true;
        self.s_dyn_fog_mul = 0.0005;
        let dtmp = 50.0f32;
        self.s_shad_gr = (l.dyn_fog - 53.0) * 0.00002;
        self.s_shad = (128.0 - l.fog_offset.max(0.0).sqrt() * 11.313708) * dtmp * 0.28;
        self.s_shad_zmul = dtmp * 0.7 / z_range as f32 * (128.0 - 128f32.sqrt() * 11.313708);
    }

    /// Positions of positional lights relative to the scene middle, depth
    /// of visible lights (`CalcXYZposForLight`) and the back-to-front
    /// order of the lights (`SortLights`).
    pub fn place_lights(&mut self, l: &Lighting, mid: [f64; 3], cam: &PaintCamera, z_range: f64, view_z: [f64; 3]) {
        self.mid = mid;
        for lv in self.lights.iter_mut() {
            let li = &l.lights[lv.idx];
            if lv.positional {
                lv.ln = [
                    (li.position[0] - mid[0]) as f32,
                    (li.position[1] - mid[1]) as f32,
                    (li.position[2] - mid[2]) as f32,
                ];
            }
            if lv.visible != 0 {
                let z = calc_z_pos_for_light(cam, lv.ln, lv.positional, view_z);
                if lv.positional {
                    lv.pos_z = z;
                    let v = 8388352.0 - cam.zc_mul * ((z as f64 / cam.step_width * cam.zcorr + 1.0).sqrt() - 1.0);
                    lv.pos_lp = (v / 256.0).clamp(0.0, 32767.0) as f32;
                } else {
                    lv.pos_lp = 0.0;
                    lv.pos_z = z_range as f32;
                }
            }
        }
        if std::env::var_os("MB3D_DEBUG_LIGHTS").is_some() {
            for lv in &self.lights {
                crate::elog!(
                    "light {}: pos {} ln {:?} col {:?} lmax {} vis {} pos_z {} pos_lp {}",
                    lv.idx + 1, lv.positional, lv.ln, lv.col, lv.lmax_l, lv.visible, lv.pos_z, lv.pos_lp
                );
            }
        }
        // SortLights: visible lights by depth, the others behind them
        self.lights.sort_by_key(|lv| if lv.visible & 6 != 0 { lv.pos_lp.round() as i32 } else { 40000 });
    }

    /// Light vector of header light `idx` (position relative to the scene
    /// middle for a positional light, else view space direction).
    pub fn light_ln(&self, idx: usize) -> Option<([f32; 3], bool)> {
        self.lights.iter().find(|l| l.idx == idx).map(|l| (l.ln, l.positional))
    }

    /// View space directions towards the lights by header light index.
    pub fn light_dirs(&self) -> Vec<(usize, [f32; 3])> {
        self.lights.iter().filter(|l| !l.positional).map(|l| (l.idx, l.ln)).collect()
    }

    /// Light vectors defined relative to the object are rotated into view space.
    pub fn rotate_object_lights(&mut self, l: &Lighting, m: &crate::math::Mat3) {
        let ms = m.map(|r| r.map(|v| v as f32));
        if let Some(d) = self.diff_map.as_mut() {
            if d.kind > 1 {
                d.rot = mat_mul(&ms, &d.rot);
            }
        }
        for (pm, rel) in self.map_lights.iter_mut().zip(&self.map_lights_rel) {
            if *rel {
                pm.rot = mat_mul(&ms, &pm.rot);
            }
        }
        for lv in self.lights.iter_mut() {
            let li = &l.lights[lv.idx];
            if li.relative_to_object && !li.positional {
                let v = lv.ln;
                let v = crate::math::rotate_vector_reverse(&[v[0] as f64, v[1] as f64, v[2] as f64], m);
                let v = crate::math::normalize(v);
                lv.ln = [v[0] as f32, v[1] as f32, v[2] as f32];
            }
        }
    }

    /// The light value part of `Interpolate2frames` (linear, `t` = weight
    /// of the second keyframe) and `Interpolate3framesBezier` (`t` = curve
    /// parameter): the values of the keyframes `keys` (made with
    /// [`LightVals::new_all`]) are combined with the weights `w`.  `self` is
    /// the frame's own light values (made with `new_all` from the keyframe
    /// the frame belongs to); what MB3D does not interpolate stays as it is.
    /// Directions of global lights are interpolated on the sphere before the
    /// lights relative to the object are rotated; positional lights are
    /// placed by the caller (their positions are interpolated in the scene).
    /// Lights that are on in any keyframe fade in or out.
    pub fn blend(&mut self, keys: &[LightVals], w: &[f64], t: f64, bezier: bool) {
        use crate::math::{bezier_quat, bezier_vec, matrix_to_quat, rotation_matrix, slerp_quat, slerp_vec};
        if keys.is_empty() || keys.len() != w.len() {
            return;
        }
        let wf: Vec<f32> = w.iter().map(|&x| x as f32).collect();
        let lin = |f: &dyn Fn(&LightVals) -> f32| -> f32 { keys.iter().zip(&wf).map(|(k, w)| f(k) * w).sum() };
        let lin_sv = |f: &dyn Fn(&LightVals) -> SVec| -> SVec {
            let mut r = [0f32; 3];
            for (k, w) in keys.iter().zip(&wf) {
                sv_add_w(&mut r, f(k), *w);
            }
            r
        };
        let rot = |ms: &[[[f32; 3]; 3]]| -> [[f32; 3]; 3] {
            let q: Vec<_> = ms.iter().map(|m| matrix_to_quat(&m.map(|r| r.map(|v| v as f64)))).collect();
            let r = if bezier && q.len() == 3 { bezier_quat(&q[0], &q[1], &q[2], t) } else { slerp_quat(&q[0], &q[1], t) };
            rotation_matrix(&r).map(|r| r.map(|v| v as f32))
        };
        // bBackBMP := L1.bBackBMP and L2.bBackBMP (and L3.bBackBMP)
        if keys.iter().any(|k| k.bg.is_none()) {
            self.bg = None;
        }
        // the singles from sGamma on, sDiff, sSpec, sIndLightReflect
        self.s_col_zmul = lin(&|k| k.s_col_zmul);
        self.s_shad_zmul = lin(&|k| k.s_shad_zmul);
        self.s_depth = lin(&|k| k.s_depth);
        self.s_shad_gr = lin(&|k| k.s_shad_gr);
        self.s_shad = lin(&|k| k.s_shad);
        self.s_amb_shad = lin(&|k| k.s_amb_shad);
        self.s_c_start = lin(&|k| k.s_c_start);
        self.s_ci_start = lin(&|k| k.s_ci_start);
        self.s_c_mul = lin(&|k| k.s_c_mul);
        self.s_ci_mul = lin(&|k| k.s_ci_mul);
        self.s_diff = lin(&|k| k.s_diff);
        self.s_spec = lin(&|k| k.s_spec);
        self.s_ind_light_reflect = lin(&|k| k.s_ind_light_reflect);
        // the colours of TLValigned
        self.depth_col = lin_sv(&|k| k.depth_col);
        self.depth_col2 = lin_sv(&|k| k.depth_col2);
        self.amb_col = lin_sv(&|k| k.amb_col);
        self.amb_col2 = lin_sv(&|k| k.amb_col2);
        self.dyn_fog_col = lin_sv(&|k| k.dyn_fog_col);
        self.dyn_fog_col2 = lin_sv(&|k| k.dyn_fog_col2);
        for i in 0..10 {
            self.col_dif[i] = lin_sv(&|k| k.col_dif[i]);
            self.col_spe[i] = lin_sv(&|k| k.col_spe[i]);
            self.col_spe_a[i] = lin(&|k| k.col_spe_a[i]);
            self.col_pos[i] = (lin(&|k| k.col_pos[i] as f32).round() as i32).clamp(0, 32767);
            self.s_c_div[i] = lin(&|k| k.s_c_div[i]).clamp(0.0, 1.0);
        }
        for i in 0..4 {
            self.col_int[i] = lin_sv(&|k| k.col_int[i]);
            self.icol_pos[i] = (lin(&|k| k.icol_pos[i] as f32).round() as i32).clamp(0, 32767);
            self.s_ic_div[i] = lin(&|k| k.s_ic_div[i]).clamp(0.0, 1.0);
        }
        // lights
        for lv in self.lights.iter_mut() {
            let kl: Vec<&LightVal> = keys.iter().filter_map(|k| k.lights.iter().find(|x| x.idx == lv.idx)).collect();
            if kl.len() != keys.len() {
                continue;
            }
            let lw = |f: &dyn Fn(&LightVal) -> f32| -> f32 { kl.iter().zip(&wf).map(|(x, w)| f(x) * w).sum() };
            lv.on = kl.iter().any(|x| x.on);
            lv.pow_func = lw(&|x| x.pow_func as f32).round() as i32;
            lv.lmax_l = lw(&|x| x.lmax_l);
            let mut c = [0f32; 3];
            for (x, w) in kl.iter().zip(&wf) {
                sv_add_w(&mut c, x.col, *w);
            }
            lv.col = c;
            if kl.iter().all(|x| !x.positional) {
                let v: Vec<[f64; 3]> = kl.iter().map(|x| x.ln.map(|c| c as f64)).collect();
                let d = if bezier && v.len() == 3 { bezier_vec(&v[0], &v[1], &v[2], t) } else { slerp_vec(&v[0], &v[1], t) };
                lv.ln = d.map(|c| c as f32);
            }
        }
        self.lights.retain(|lv| lv.on);
        // gamma
        let d1: f32 = keys.iter().zip(&wf).map(|(k, w)| k.s_gamma * w * k.gamma_h as f32).sum();
        self.s_gamma = d1.abs().min(1.0);
        self.gamma_h = if d1.abs() < 0.005 {
            0
        } else if d1 < 0.0 {
            -1
        } else {
            1
        };
        self.s_dyn_fog_mul = lin(&|k| k.s_dyn_fog_mul);
        self.s_roughness_factor = lin(&|k| k.s_roughness_factor);
        self.s_diffuse_shadowing = lin(&|k| k.s_diffuse_shadowing);
        // picture rotations
        if self.bg.is_some() {
            let ms: Vec<_> = keys.iter().map(|k| k.bg.as_ref().unwrap().rot).collect();
            self.bg.as_mut().unwrap().rot = rot(&ms);
        }
        for (j, &idx) in self.map_lights_idx.clone().iter().enumerate() {
            let ms: Option<Vec<_>> = keys
                .iter()
                .map(|k| k.map_lights_idx.iter().position(|&i| i == idx).map(|p| k.map_lights[p].rot))
                .collect();
            if let Some(ms) = ms {
                self.map_lights[j].rot = rot(&ms);
            }
        }
        if self.diff_map.is_some() && keys.iter().all(|k| k.diff_map.is_some()) {
            let dm: Vec<&DiffMap> = keys.iter().map(|k| k.diff_map.as_ref().unwrap()).collect();
            let r = if dm.len() > 1 { 1 } else { 0 };
            // offsets wrap around (period 1), towards the middle keyframe
            let mut off = [0f32; 2];
            for (c, o) in off.iter_mut().enumerate() {
                let d2 = dm[r].off[c];
                *o = dm
                    .iter()
                    .zip(&wf)
                    .map(|(d, w)| {
                        let mut v = d.off[c];
                        if (v - d2).abs() > 0.5 {
                            v += if v < d2 { 1.0 } else { -1.0 };
                        }
                        v * w
                    })
                    .sum();
            }
            let ms: Vec<_> = dm.iter().map(|d| d.rot).collect();
            let new_rot = rot(&ms);
            let ang: Vec<f32> = dm.iter().map(|d| d.rot_sin.atan2(if d.rot_cos == 0.0 { 1e-30 } else { d.rot_cos })).collect();
            let a: f32 = ang
                .iter()
                .zip(&wf)
                .map(|(&a, w)| {
                    let a = if (a - ang[r]).abs() > std::f32::consts::PI {
                        a + (ang[r] - a).signum() * 2.0 * std::f32::consts::PI
                    } else {
                        a
                    };
                    a * w
                })
                .sum();
            let scale = lin(&|k| k.diff_map.as_ref().unwrap().scale);
            let d = self.diff_map.as_mut().unwrap();
            d.off = off;
            d.rot = new_rot;
            d.rot_sin = a.sin();
            d.rot_cos = a.cos();
            d.scale = scale;
        }
    }

    /// The surface palette colour (0..1) and the position in the palette
    /// (0..1) for a colouring value, as the mesh export uses them
    /// (BulbTracer2's `CalcColors` and `CalcColorsIdx`).
    pub fn palette_color(&self, si_gradient: u16, otrap: u16) -> ([f32; 3], f32) {
        let si = SiLight { si_gradient, otrap, ..Default::default() };
        let (dif, _) = self.calc_colors(&si, 0.0);
        let ir = if self.col_on_otrap { (otrap & 0x7FFF) as f32 } else { si_gradient as f32 };
        let mut ir = (((ir - self.s_c_start) * self.s_c_mul) * 16384.0).clamp(-1e9, 1e9).round() as i32;
        let idx = if self.col_cycling {
            ir &= 32767;
            ir as f32 / self.col_pos[9].max(1) as f32
        } else if ir < 0 {
            0.0
        } else if ir >= self.col_pos[9] {
            1.0
        } else {
            ir as f32 / self.col_pos[9].max(1) as f32
        };
        // internal gamma 2 stores (v / 255)^2
        let c = if self.sqr { dif.map(|v| v.max(0.0).sqrt()) } else { dif };
        (c, idx)
    }

    /// `CalcColors`
    pub(crate) fn calc_colors(&self, si: &SiLight, idif0: f32) -> (SVec, SVec) {
        let (d, s, _) = self.calc_colors_alpha(si, idif0);
        (d, s)
    }

    /// `CalcColors` with the alpha of the specular colour (`iSpe[3]`, the
    /// transparency of the Monte Carlo renderer).
    pub(crate) fn calc_colors_alpha(&self, si: &SiLight, idif0: f32) -> (SVec, SVec, f32) {
        let ir = if self.col_on_otrap { (si.otrap & 0x7FFF) as f32 } else { si.si_gradient as f32 };
        let mut ir = (((ir - self.s_c_start) * self.s_c_mul + idif0) * 16384.0).clamp(-1e9, 1e9).round() as i32;
        if self.col_cycling {
            ir &= 32767;
        } else if ir < 0 {
            return (self.col_dif[0], self.col_spe[0], self.col_spe_a[0]);
        } else if ir >= self.col_pos[9] {
            return (self.col_dif[9], self.col_spe[9], self.col_spe_a[9]);
        }
        let mut il2 = 5usize;
        if self.col_pos[il2] < ir {
            loop {
                il2 += 1;
                if il2 == 10 || self.col_pos[il2] >= ir {
                    break;
                }
            }
        } else {
            while il2 > 1 && self.col_pos[il2 - 1] >= ir {
                il2 -= 1;
            }
        }
        if self.no_col_ipol {
            return (self.col_dif[il2 - 1], self.col_spe[il2 - 1], self.col_spe_a[il2 - 1]);
        }
        let il1 = il2 - 1;
        let il2 = if il2 > 9 { 0 } else { il2 };
        let t = (ir - self.col_pos[il1]) as f32 * self.s_c_div[il1];
        (
            lerp(self.col_dif[il2], self.col_dif[il1], t),
            lerp(self.col_spe[il2], self.col_spe[il1], t),
            self.col_spe_a[il1] + t * (self.col_spe_a[il2] - self.col_spe_a[il1]),
        )
    }

    /// `CalcColorsInside`: diffuse colour and specular amount (alpha)
    pub(crate) fn calc_colors_inside(&self, si: &SiLight, idif0: f32) -> (SVec, SVec) {
        let mut ir =
            (((si.si_gradient as f32 - self.s_ci_start) * self.s_ci_mul + idif0) * 16384.0).round() as i32;
        let (dif, spe);
        if self.col_cycling {
            ir &= 32767;
        }
        if !self.col_cycling && ir < 0 {
            dif = self.col_int[0];
            spe = self.col_int_spec[0];
        } else if !self.col_cycling && ir > self.icol_pos[3] {
            dif = self.col_int[3];
            spe = self.col_int_spec[3];
        } else {
            let mut il2 = 1;
            while il2 < 4 && self.icol_pos[il2] < ir {
                il2 += 1;
            }
            if self.no_col_ipol {
                dif = self.col_int[il2 - 1];
                spe = self.col_int_spec[il2 - 1];
            } else {
                let il1 = il2 - 1;
                let il2 = il2 & 3;
                let t = (ir - self.icol_pos[il1]) as f32 * self.s_ic_div[il1];
                dif = lerp(self.col_int[il2], self.col_int[il1], t);
                spe = self.col_int_spec[il2] + t * (self.col_int_spec[il1] - self.col_int_spec[il2]);
            }
        }
        (dif, [spe; 3])
    }

    /// The diffuse colour map lookup (`iColOnOT > 1`).
    pub(crate) fn diff_map_color(&self, d: &DiffMap, si: &SiLight, lns: SVec, obj_pos: SVec, d_rough: f32, cam: &PaintCamera) -> SVec {
        let sqr = self.sqr;
        if d.kind > 1 {
            let c = if d.kind > 2 {
                // GetDiffMapWrap3D
                let mut p = [
                    (obj_pos[0] as f64 + self.mid[0]) * d.scale as f64,
                    (obj_pos[1] as f64 + self.mid[1]) * d.scale as f64,
                    (obj_pos[2] as f64 + self.mid[2]) * d.scale as f64,
                ];
                let mut n = cam.to_abs(lns);
                let l1 = n[0].abs() + n[1].abs() + n[2].abs();
                n = n.map(|v| (v / l1.max(1e-30)).abs());
                let neg = |v: f64| if v < 0.0 { v + 1.0 - v.round_ties_even() } else { v };
                let (ox, oy) = (d.off[0] as f64, d.off[1] as f64);
                if d.kind > 3 {
                    p = p.map(neg);
                    let px = |a: f64, b: f64| d.map.pixel_t(frac(a + ox), frac(b + oy), 1, sqr);
                    let mut r = sv_scale(px(p[1], p[2]), n[0]);
                    sv_add_w(&mut r, px(p[0], p[2]), n[1]);
                    sv_add_w(&mut r, px(p[0], p[1]), n[2]);
                    r
                } else {
                    let sn = p.map(|v| v.sin());
                    let (n0, n1, n2) = (n[0] as f64, n[1] as f64, n[2] as f64);
                    let x = neg(sn[1] * n0 + sn[0] * (n1 + n2) + ox);
                    let y = neg(sn[2] * (n0 + n1) + sn[1] * n2 + oy);
                    d.map.pixel_t(frac(x), frac(y), 1, sqr)
                }
            } else {
                d.map.sphere_pixel_t(lns, Some(&d.rot), sqr)
            };
            let mut r = sv_scale(c, 1.0 - d_rough);
            sv_add_w(&mut r, d.map.avg, d_rough / 255.0);
            r
        } else {
            let t1 = (si.otrap & 0x7FFF) as f32 * 3.05186851e-5 - 0.5;
            let t2 = si.si_gradient as f32 * 3.05186851e-5 - 0.5;
            let x = (d.rot_cos * t1 + d.rot_sin * t2 + d.off[0]) * d.scale;
            let y = (d.rot_cos * t2 - d.rot_sin * t1 + d.off[1]) * d.scale;
            d.map.pixel_t(frac(x as f64), frac(y as f64), 1, sqr)
        }
    }

    /// `CalcPixelColor2` for pixel (x, y). Returns RGB.
    pub fn pixel_color(&self, si: &SiLight, x: i32, y: i32, cam: &PaintCamera) -> [u8; 3] {
        self.pixel_color_s(si, x, y, cam)
    }

    /// [`Self::pixel_color`] in 8 or 16 bit.
    pub fn pixel_color_s<S: Sample>(&self, si: &SiLight, x: i32, y: i32, cam: &PaintCamera) -> [S; 3] {
        let plv = self.plv_for_pixel(si, x, y, cam);
        let sh = self.shade(si, &plv, cam, &ShadeOpts::default());
        let out = sh.light;
        let sqr = self.sqr;
        let mut c = if sqr {
            out.map(|v| (v * 255.0).clamp(0.0, 65025.0).sqrt())
        } else {
            [out[0].clamp(0.0, 255.0), out[1].clamp(0.0, 255.0), out[2].clamp(0.0, 255.0)]
        };
        if self.gamma_h != 0 {
            let g = if self.gamma_h > 0 {
                [(c[0] * 255.0).sqrt(), (c[1] * 255.0).sqrt(), (c[2] * 255.0).sqrt()]
            } else {
                [c[0] * c[0] / 255.0, c[1] * c[1] / 255.0, c[2] * c[2] / 255.0]
            };
            c = lerp(g, c, self.s_gamma);
        }
        [S::from_paint(c[0]), S::from_paint(c[1]), S::from_paint(c[2])]
    }

    /// The paint values of a pixel of the G-buffer (`CalcViewVec`,
    /// `CalcObjPos`, `PreCalcDepthCol`).
    pub(crate) fn plv_for_pixel(&self, si: &SiLight, x: i32, y: i32, cam: &PaintCamera) -> Plv {
        let x_pos = (x + 1) as f32 / cam.width as f32;
        let y_pos = y as f32 / cam.height as f32;
        let view = cam.view_vec(x_pos, y_pos, cam.aspect);
        let ys = match self.depth_func {
            1 => y_pos * y_pos,
            0 => y_pos,
            _ => y_pos.max(0.0).sqrt(),
        };
        let zpos_word = si.zpos();
        let z1 = if zpos_word > 32767 {
            ((8388352.0 / cam.zc_mul + 1.0).powi(2) - 1.0) * cam.step_width / cam.zcorr
        } else {
            (((8388352 - (si.zpos_fine >> 8) as i64) as f64 / cam.zc_mul + 1.0).powi(2) - 1.0) * cam.step_width / cam.zcorr
        };
        let z_pos = (z1 + cam.zz_stmit_dif) as f32;
        let abs_view = cam.to_abs(view);
        let cam_pos = cam.cam_pos(x as f32, y as f32);
        Plv {
            view,
            abs_view,
            cam_pos,
            obj_pos: sv_add(cam_pos, sv_scale(abs_view, z1 as f32)),
            z_pos,
            zpos_dyn_fog: z_pos,
            x_pos,
            y_pos,
            dep_c: lerp(self.depth_col2, self.depth_col, ys),
        }
    }

    /// `CalcPixelColor2` / `CalcPixelColorSvec(Trans)`: the light (0..255
    /// scale, before clipping and gamma) a view ray `plv` gets from the
    /// G-buffer value `si`, plus the transmission factor and the surface
    /// colours used by the reflection pass.
    pub(crate) fn shade(&self, si: &SiLight, plv: &Plv, cam: &PaintCamera, o: &ShadeOpts) -> Shaded {
        let view = plv.view;
        let abs_view = plv.abs_view;
        let mut dep_c = plv.dep_c;
        let zpos_word = si.zpos();
        let plv_zpos = plv.z_pos;
        if self.amb_rel_obj {
            let w = abs_view[1].clamp(-1.0, 1.0).asin() * std::f32::consts::FRAC_1_PI + 0.5;
            dep_c = lerp(self.depth_col2, self.depth_col, w);
        }
        let sqr = self.sqr;
        let cam_pos = plv.cam_pos;
        let obj_pos = plv.obj_pos;
        let mut result = [1f32; 3];
        let mut spe_out = [0f32; 3];
        let mut spe_a = 0f32;
        let mut dif_out = [0f32; 3];

        let mut out: SVec;
        let mut dtmp;
        if zpos_word < 32768 {
            let d_rough = (si.zpos_fine & 0xFF) as f32 * self.s_roughness_factor;
            // calcAmbshadow
            let shadow = if si.amb_shadow >= 16383 { 1.0 } else { si.amb_shadow as f32 / 16383.0 };
            let mut d_amb_sh = if self.s_amb_shad < 1.0 {
                1.0 - self.s_amb_shad * shadow
            } else {
                let a = 1.0 - shadow;
                a + (self.s_amb_shad - 1.0) * (a * a - a)
            };
            if sqr {
                d_amb_sh = if o.svec { d_amb_sh * d_amb_sh } else { (d_amb_sh * d_amb_sh + d_amb_sh) * 0.5 };
            }
            let lns: SVec = [
                si.normal[0] as f32 * 3.0518509476e-5,
                si.normal[1] as f32 * 3.0518509476e-5,
                si.normal[2] as f32 * 3.0518509476e-5,
            ];
            let mut li_dif = [0f32; 3]; // LiLSDAI[1]
            let mut li_spe = [0f32; 3];
            let mut li_amb = [0f32; 3]; // LiLSDAI[3]
            let d_fog = self.s_diffuse_shadowing * (d_amb_sh - 1.0) + 1.0;
            for lv in &self.lights {
                let soft = lv.hs_mask == u16::MAX;
                let no_hs = soft || (si.shadow & lv.hs_mask) == 0 || !lv.hs_calced;
                let soft_mul = if soft { (si.shadow >> 10) as f32 * (1.0 / 63.0) } else { 1.0 };
                let (ln, att) = if lv.positional {
                    let v = sv_sub(lv.ln, obj_pos);
                    let d2 = sv_dot(v, v);
                    if d2 > lv.lmax_l {
                        continue;
                    }
                    (cam.to_view(sv_scale(v, 1.0 / d2.sqrt())), 1.0 / (d2 + 1e-30))
                } else {
                    (lv.ln, 1.0)
                };
                let dot = lns[0] * ln[0] + lns[1] * ln[1] + lns[2] * ln[2];
                let mut d = if sqr { get_cos_tab_val_sqr(lv.diff_func, dot, d_rough) } else { get_cos_tab_val(lv.diff_func, dot, d_rough) } * att;
                sv_add_w(&mut li_amb, lv.col, d);
                if no_hs {
                    d *= if lv.sub_amb_sh { d_amb_sh } else { d_fog };
                    d *= soft_mul;
                    sv_add_w(&mut li_dif, lv.col, d);
                }
                // DotOf2VecNormalize: reflect(view, n) . light
                let d2 = 2.0 * (lns[0] * view[0] + lns[1] * view[1] + lns[2] * view[2]);
                let ds = ln[0] * (view[0] - lns[0] * d2) + ln[1] * (view[1] - lns[1] * d2) + ln[2] * (view[2] - lns[2] * d2);
                if ds > 0.0 {
                    let pf = lv.pow_func as f32;
                    let mut t2 = (att + (d_rough * 2.0).min(1.0) * (1.0 / pf - att)) * self.s_spec;
                    if t2 > 0.0 {
                        sv_add_w(&mut li_amb, lv.col, t2 / pf);
                        if no_hs {
                            t2 *= fast_int_pow(ds, lv.pow_func);
                            t2 *= if lv.sub_amb_sh { d_amb_sh } else { d_fog };
                            t2 *= soft_mul;
                            sv_add_w(&mut li_spe, lv.col, t2);
                        }
                    }
                }
            }
            // light maps: ambient light from an image, looked up by the normal
            for lm in &self.map_lights {
                let px = lm.map.sphere_pixel_t(lns, Some(&lm.rot), sqr);
                let px = sv_scale(px, 255.0 * lm.intensity);
                // (LMavrgColSqr is never set in MB3D)
                let c = lerp(if sqr { [0.0; 3] } else { lm.map.avg }, px, d_rough);
                sv_add_w(&mut li_amb, c, 1.0);
                sv_add_w(&mut li_dif, c, d_amb_sh);
            }
            let idif0 = self.s_col_zmul * plv_zpos;
            let (mut idif, ispe, ia) = if si.si_gradient > 32767 {
                let (d, s) = self.calc_colors_inside(si, idif0);
                (d, s, s[0])
            } else {
                self.calc_colors_alpha(si, idif0)
            };
            if let Some(d) = &self.diff_map {
                let pal = idif;
                idif = self.diff_map_color(d, si, lns, obj_pos, d_rough, cam);
                if self.yc_comb {
                    idif = sv_scale(pal, luma(idif) / (0.01 + luma(pal)));
                }
            }
            spe_out = ispe;
            spe_a = ia;
            dif_out = idif;
            let mut s_diff = self.s_diff;
            if let Some(sr) = o.scale_amb_diff_down {
                let t = 1.0 - ia * sr;
                d_amb_sh *= t;
                s_diff *= t;
            }
            let mut iamb = if let Some(bs) = &self.bg_small {
                // the small background picture as ambient light
                let px = bs.map.sphere_pixel_t(cam.to_abs(lns), Some(&bs.rot), sqr);
                let mut a = sv_scale(px, 255.0 * bs.intensity);
                let mut t = (1.0 - (1.0 - 28000.0 * self.s_depth).max(0.0)).max(0.0);
                if self.far_fog {
                    t *= t;
                }
                if self.bg_add_light {
                    sv_add_w(&mut a, dep_c, t);
                } else {
                    a = lerp(a, dep_c, 1.0 - t);
                }
                sv_scale(a, d_amb_sh)
            } else {
                // ambient light top / bottom
                let ny = if self.amb_rel_obj { cam.to_abs(lns)[1] } else { lns[1] };
                let t2 = (ny * 0.5 + 0.5) * d_amb_sh;
                let t1 = d_amb_sh - t2;
                sv_add(sv_scale(self.amb_col, t1), sv_scale(self.amb_col2, t2))
            };
            iamb = sv_mul(iamb, idif);
            let dif_s = sv_scale(idif, s_diff); // LiLSDAI[0]
            let total = sv_add(sv_add(iamb, sv_mul(dif_s, li_dif)), sv_mul(ispe, li_spe)); // [1]
            if !self.ex_mode {
                // CalcTotalLight1
                let l3 = sv_add(iamb, sv_scale(sv_mul(dif_s, li_amb), d_amb_sh * if sqr { 0.11 } else { 0.3 }));
                let st = (1.0 - d_amb_sh).max(0.0);
                let st2 = st * st * st * st;
                let refl = [
                    idif[0] * st2 * self.s_ind_light_reflect + (st - st2),
                    idif[1] * st2 * self.s_ind_light_reflect + (st - st2),
                    idif[2] * st2 * self.s_ind_light_reflect + (st - st2),
                ];
                out = sv_add(total, sv_mul(l3, sv_mul(idif, refl)));
            } else {
                // CalcTotalLight2
                let st = (1.0 - d_amb_sh).max(0.0);
                let st = st * st * self.s_ind_light_reflect;
                let st2 = luma(total) + 4.0;
                out = sv_add(total, sv_scale(sv_mul(total, idif), st));
                out = sv_scale(out, (st2 + (st2 + 128.0) * (st + 0.11) * 0.04) / (luma(out).max(0.0) + 3.0));
            }
            dtmp = ((zpos_word as f32 - 28000.0) * self.s_depth + 1.0).max(0.0);
        } else {
            out = match &self.bg {
                Some(bg) => {
                    let px = if self.bg_direct {
                        bg.map.pixel_t(plv.x_pos, plv.y_pos, 0, sqr)
                    } else {
                        bg.map.sphere_pixel_t(abs_view, Some(&bg.rot), sqr)
                    };
                    sv_scale(px, 255.0 * bg.intensity)
                }
                None => dep_c,
            };
            // with the reflection pass the depth fog towards infinity is
            // added exactly (`60768 - Zpos`; 1 - 28000 sDepth for Zpos = 32768)
            dtmp = (1.0 - (60768 - zpos_word as i32) as f32 * self.s_depth).max(0.0);
        }
        if o.inside_trans.is_none() {
            if sqr {
                if dtmp < 1.0 {
                    let mut t = (1.0 - dtmp) * (1.0 - dtmp);
                    if self.far_fog {
                        t *= t;
                    }
                    dtmp = 1.0 - t;
                }
            } else if self.far_fog && dtmp < 1.0 {
                dtmp = 1.0 - (1.0 - dtmp) * (1.0 - dtmp);
            }
            if zpos_word < 32768 || self.bg.is_none() || !self.bg_add_light {
                out = sv_scale(out, dtmp);
            }
            // dynamic fog
            let ir = if self.vol_light { convert_vlight(si.shadow) as f32 } else { (si.shadow & 0x3FF) as f32 };
            let mut dfog = (ir - self.s_shad - self.s_shad_zmul * plv.zpos_dyn_fog) * self.s_shad_gr;
            if self.dfog_options & 2 != 0 {
                dfog = dfog.max(0.0);
            }
            let mut dtmp3 = (ir * self.s_dyn_fog_mul).min(1.0) * dfog;
            out = sv_add(out, sv_scale(dep_c, (1.0 - dtmp).max(0.0)));
            if self.dfog_options & 1 != 0 {
                dfog = dfog.clamp(0.0, 1.0);
                dtmp3 = dtmp3.clamp(0.0, 1.0);
                out = sv_scale(out, 1.0 - dfog);
                result = sv_scale(result, 1.0 - dfog);
            }
            out = sv_add(
                out,
                sv_add(sv_scale(self.dyn_fog_col, dfog - dtmp3), sv_scale(self.dyn_fog_col2, dtmp3)),
            );
            result = sv_scale(result, dtmp.min(1.0));
        }
        // visible lights
        let il1 = if zpos_word < 32768 { 32768 - zpos_word as i32 } else { zpos_word as i32 };
        let d_rough_v = 1.0 / il1 as f32;
        for (li, lv) in self.lights.iter().enumerate() {
            if lv.visible == 0 {
                continue;
            }
            let pos = lv.positional;
            let mut behind = false;
            let mut flux = if pos {
                sqr_dist_sv(sv_sub(cam_pos, lv.ln), abs_view)
            } else {
                if zpos_word < 32768 {
                    continue;
                }
                (1.0 - sv_dot(lv.ln, view)).max(0.0)
            };
            let mut transp = lv.lmax_l * 1e-8;
            if flux >= transp {
                continue;
            }
            if pos {
                // light behind the viewer?
                if sv_dot(sv_sub(lv.ln, cam_pos), abs_view) < 0.0 {
                    continue;
                }
                if zpos_word < 32768 {
                    // light behind the object?
                    let t = -(transp - flux).sqrt();
                    behind = if abs_view[0].abs() > 0.5 {
                        t > (obj_pos[0] - lv.ln[0]) / abs_view[0]
                    } else if abs_view[1].abs() > 0.5 {
                        t > (obj_pos[1] - lv.ln[1]) / abs_view[1]
                    } else {
                        t > (obj_pos[2] - lv.ln[2]) / abs_view[2]
                    };
                }
            }
            if behind {
                continue;
            }
            let (pos_z, pos_lp) = match o.light_z.and_then(|z| z.get(li)) {
                Some(&(z, lp)) if pos => (z, lp),
                _ => (lv.pos_z, lv.pos_lp),
            };
            pos_light_shape(&mut flux, &mut transp, lv.visible, pos, sqr);
            if let Some((din, absorp)) = o.inside_trans {
                // inside transparent material: the light is coloured by the way through it
                let t = 1.0 - (1.0 + (pos_lp - 28000.0) * self.s_depth).max(0.0);
                let c = [din[0].powf(t * absorp), din[1].powf(t * absorp), din[2].powf(t * absorp)];
                out = sv_add(out, sv_mul(c, sv_scale(lv.col, flux)));
            } else {
                let mut fog_at = 1.0 - (1.0 + (pos_lp - 28000.0) * self.s_depth).max(0.0);
                if sqr && fog_at > 0.0 {
                    fog_at *= fog_at;
                }
                if self.far_fog && fog_at > 0.0 {
                    fog_at *= fog_at;
                }
                flux *= 1.0 - fog_at * 0.9;
                if lv.visible == 2 || lv.visible == 8 {
                    out = sv_add(out, sv_scale(lv.col, flux));
                } else {
                    let fog_at = fog_at.max(0.0);
                    let il = if self.vol_light { convert_vlight(si.shadow) as f32 } else { (si.shadow & 0x3FF) as f32 };
                    let k = il * (32768.0 - pos_lp) * d_rough_v;
                    let dfog = (k - self.s_shad - self.s_shad_zmul * (pos_z + cam.zz_stmit_dif as f32))
                        * self.s_shad_gr
                        * (1.0 - transp)
                        * (1.0 - fog_at);
                    let damb = (k * self.s_dyn_fog_mul).min(1.0) * dfog;
                    out = sv_add(
                        sv_add(sv_scale(out, transp), sv_scale(dep_c, fog_at * (1.0 - transp))),
                        sv_add(
                            sv_add(sv_scale(self.dyn_fog_col, dfog - damb), sv_scale(self.dyn_fog_col2, damb)),
                            sv_scale(lv.col, flux),
                        ),
                    );
                }
            }
            result = sv_scale(result, transp.min(1.0));
        }
        Shaded { light: out, result, spe: spe_out, spe_a, dif: dif_out }
    }
}

/// The view ray of a shaded point (`TPaintLightVals`): directions in view
/// and scene space, camera (ray start) and object position relative to the
/// scene middle, z for the colour and the dynamic fog, image position for
/// a direct background picture, the depth colour.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Plv {
    pub(crate) view: SVec,
    pub(crate) abs_view: SVec,
    pub(crate) cam_pos: SVec,
    pub(crate) obj_pos: SVec,
    pub(crate) z_pos: f32,
    pub(crate) zpos_dyn_fog: f32,
    pub(crate) x_pos: f32,
    pub(crate) y_pos: f32,
    pub(crate) dep_c: SVec,
}

/// Options of [`LightVals::shade`] for the reflection pass.
#[derive(Default)]
pub(crate) struct ShadeOpts<'a> {
    /// `bScaleAmbDiffDown`: ambient and diffuse light are reduced by the
    /// transparency (alpha) times this light amount
    pub(crate) scale_amb_diff_down: Option<f32>,
    /// inside transparent material (`bDivOptions`): no depth or dynamic
    /// fog; visible lights coloured by (diffuse colour, absorption)
    pub(crate) inside_trans: Option<(SVec, f32)>,
    /// depth (`sPosLightZpos`, `sPosLP`) of positional lights as seen from
    /// the start of a reflected ray, per entry of `lights`
    pub(crate) light_z: Option<&'a [(f32, f32)]>,
    /// `CalcPixelColorSvec(Trans)` (the reflection pass) rather than
    /// `CalcPixelColor2` / `CalcPixelColorSqr`: with internal gamma 2 the
    /// ambient shadow is squared (not the mean of it and its square)
    pub(crate) svec: bool,
}

/// Result of [`LightVals::shade`].
pub(crate) struct Shaded {
    /// light, 0..255 scale (not clipped)
    pub(crate) light: SVec,
    /// share of the light that passes the fogs and visible lights
    pub(crate) result: SVec,
    /// specular colour and its alpha, diffuse colour of the surface
    pub(crate) spe: SVec,
    pub(crate) spe_a: f32,
    pub(crate) dif: SVec,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cos_tab_monotonic() {
        let a = get_cos_tab_val(0, 1.0, 0.0);
        let b = get_cos_tab_val(0, 0.5, 0.0);
        let c = get_cos_tab_val(0, -0.5, 0.0);
        assert!(a > b && b > c, "{a} {b} {c}");
        assert!((a - 1.0).abs() < 0.1);
    }

    #[test]
    fn default_colors() {
        let l = Lighting::default();
        assert_eq!(l.palette[0].diffuse, [0xE1, 0xA0, 0x59]);
        assert_eq!(l.lights[0].color, [0xFF, 0xE8, 0xA0]);
    }
}
