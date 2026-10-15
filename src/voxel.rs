//! Voxel export (VoxelExport.pas): the object is cut into a stack of
//! slices, each saved as a 1 bit PNG (white = object), for 3D printing or
//! for building meshes in other programs.  The settings are MB3D's `.m3v`
//! project (`TM3Vfile`).
//!
//! The slices are taken in the space of the camera (or the axes, with
//! "default orientation"): the stack covers 2.2 / zoom units in z, the
//! slice size follows from the scales, and the offsets move the box.  A
//! point belongs to the object when its distance estimate is below the DE
//! threshold (default) or when the iterations reach the maximum.

use crate::calc::{CalcParams, Marcher};
use crate::math::{normalise_matrix_to, Mat3, Vec3, IDENTITY3};
use crate::scene::Scene;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

const HEADER_SIZE: usize = 840;
const ADDON_SIZE: usize = 8 + 6 * 188;
/// Size of `TM3Vfile` before the header: the settings (204 bytes) and the
/// output folder (1024 bytes).
const M3V_SETTINGS: usize = 204 + 1024;
/// `iVoxelVersion`
const VOXEL_VERSION: i32 = 4;

/// How the object is found (`ObjectD`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectTest {
    /// iterations reach the maximum
    Iterations = 0,
    /// distance estimate below the threshold
    De = 1,
}

/// The settings of a voxel export (`TM3Vfile`).
#[derive(Clone, Debug)]
pub struct VoxelParams {
    /// Offsets of the box in scene units (`Xoff`, `Yoff`, `Zoff`)
    pub offset: [f64; 3],
    /// Box proportions (`Xscale`, `Yscale`, `Zscale`); z = 1 covers 2.2 / zoom
    pub scale: [f64; 3],
    /// Number of slices (`Zslices`)
    pub slices: u32,
    pub test: ObjectTest,
    /// Maximum iterations (`MaxIts`)
    pub max_its: i32,
    /// DE threshold in voxels (`DE`)
    pub de: f64,
    /// In and outside rendering: hollow object between these and the maxima
    /// (`MinIts`, `MinDE`)
    pub min_its: i32,
    pub min_de: f64,
    /// White outside, black object (`WhiteOutside`)
    pub white_outside: bool,
    /// Slice along the axes instead of the view (`UseDefaultOrientation`)
    pub default_orientation: bool,
    /// File index with 6 digits (`LeadingZeros`)
    pub leading_zeros: bool,
    /// Output folder stored in the project (`OutputFolderC`)
    pub output_folder: String,
    /// Image width of the scene the settings were made from (`OrigWidth`)
    pub orig_width: i32,
}

impl VoxelParams {
    /// `Button4Click` ("import parameters from main"): settings for a scene.
    pub fn from_scene(sc: &Scene, slices: u32) -> VoxelParams {
        let slices = slices.max(2);
        let d = if sc.vary_de_stop_on_fov {
            (1.0 + 0.3 / sc.de_stop.max(1e-6)) * sc.fov_y.max(0.0).to_radians() * (sc.zoom * sc.width as f64)
                / (sc.height.max(1) as f64 * 2.1345)
        } else {
            0.0
        };
        let de = (sc.de_stop * (1.0 + (sc.mid[2] - sc.z_start) * d) * slices as f64 / sc.width.max(1) as f64).max(0.2);
        VoxelParams {
            offset: [0.0; 3],
            scale: [1.0; 3],
            slices,
            test: ObjectTest::De,
            max_its: sc.iterations,
            de,
            min_its: sc.iterations / 2,
            min_de: de * 0.254,
            white_outside: false,
            default_orientation: false,
            leading_zeros: true,
            output_folder: String::new(),
            orig_width: sc.width,
        }
    }

    /// `CalcImageSize`: width and height of the slices.
    pub fn size(&self) -> (usize, usize) {
        let d = self.slices as f64 / self.scale[2].max(1e-40);
        (((self.scale[0] * d).round() as usize).max(1), ((self.scale[1] * d).round() as usize).max(1))
    }

    /// File of slice `nr` (1-based): `<name><index>.png`.
    pub fn slice_file(&self, dir: &Path, name: &str, nr: u32) -> PathBuf {
        let idx = if self.leading_zeros { format!("{nr:06}") } else { nr.to_string() };
        dir.join(format!("{name}{idx}.png"))
    }
}

/// Positions of the voxels: `start(nr) + m[1] * y + m[0] * x`.
#[derive(Clone, Debug)]
pub struct Grid {
    pub width: usize,
    pub height: usize,
    pub slices: u32,
    /// rows: the steps in x, y, z (length = voxel size)
    pub m: Mat3,
    /// position of voxel (0, 0) of slice 0
    pub origin: Vec3,
}

impl Grid {
    /// `StartSlice`: the voxel matrix and the start of the stack.
    pub fn new(sc: &Scene, vp: &VoxelParams) -> Grid {
        let (w, h) = vp.size();
        let n = vp.slices.max(2);
        let d = 2.2 / (sc.zoom * vp.scale[2] * (n - 1) as f64);
        let r = if vp.default_orientation { IDENTITY3 } else { normalise_matrix_to(1.0, &sc.vgrads) };
        let m = r.map(|row| row.map(|v| v * d));
        let mut o = sc.mid;
        let f = [w as f64 * -0.5 + vp.offset[0] / d, h as f64 * -0.5 + vp.offset[1] / d, n as f64 * -0.5 + vp.offset[2] / d];
        for (k, fk) in f.iter().enumerate() {
            for c in 0..3 {
                o[c] += m[k][c] * fk;
            }
        }
        Grid { width: w, height: h, slices: n, m, origin: o }
    }

    pub fn pos(&self, x: f64, y: f64, nr: f64) -> Vec3 {
        let mut p = self.origin;
        for c in 0..3 {
            p[c] += self.m[0][c] * x + self.m[1][c] * y + self.m[2][c] * nr;
        }
        p
    }
}

/// The scene as the export calculates it (`VHeader` with the slice size,
/// the DE threshold and the maximum iterations).
pub fn export_scene(sc: &Scene, vp: &VoxelParams) -> Scene {
    let mut s = sc.clone();
    let (w, h) = vp.size();
    s.width = w as i32;
    s.height = h as i32;
    s.tiling = None;
    s.calc_rect = None;
    s.light_blend = None;
    s.de_stop = vp.de;
    s.iterations = vp.max_its.max(1);
    s
}

/// One voxel (`TFVoxelExportCalcThread.Execute`).
fn voxel(m: &mut Marcher, p: &CalcParams, vp: &VoxelParams, pos: Vec3) -> bool {
    match vp.test {
        ObjectTest::Iterations => {
            let it = m.iterations_at(pos);
            if p.in_and_outside {
                it < p.max_it && it >= vp.min_its
            } else {
                (it >= p.max_it) != p.inside_rendering
            }
        }
        ObjectTest::De => {
            let d = m.de_at_point(pos, false);
            d < p.de_stop as f64 && !(p.in_and_outside && d < vp.min_de)
        }
    }
}

/// Calculates slice `nr` (1-based): true = object.
pub fn calc_slice(
    p: &CalcParams,
    grid: &Grid,
    vp: &VoxelParams,
    nr: u32,
    threads: usize,
    cancel: &(dyn Fn() -> bool + Sync),
) -> Vec<bool> {
    let (w, h) = (grid.width, grid.height);
    let threads = threads.clamp(1, h.max(1));
    let mut out = vec![false; w * h];
    let rows = std::sync::Mutex::new(&mut out);
    std::thread::scope(|s| {
        for t in 0..threads {
            let rows = &rows;
            s.spawn(move || {
                let mut m = Marcher::new(p, 1);
                let mut buf = vec![false; w];
                let mut y = t;
                while y < h {
                    if cancel() {
                        return;
                    }
                    for (x, b) in buf.iter_mut().enumerate() {
                        *b = voxel(&mut m, p, vp, grid.pos(x as f64, y as f64, nr as f64));
                    }
                    rows.lock().unwrap()[y * w..(y + 1) * w].copy_from_slice(&buf);
                    y += threads;
                }
            });
        }
    });
    out
}

/// Renders all slices into `dir` (`Button2Click` / `Timer1Timer`).  Calls
/// `progress(slice, slices)` after each slice; returns the slice count.
pub fn export(
    sc: &Scene,
    vp: &VoxelParams,
    dir: &Path,
    name: &str,
    threads: usize,
    progress: &dyn Fn(u32, u32),
    cancel: &(dyn Fn() -> bool + Sync),
) -> Result<u32, String> {
    let s = export_scene(sc, vp);
    let p = CalcParams::new(&s)?;
    let grid = Grid::new(&s, vp);
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let threads = if threads > 0 { threads } else { std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4) };
    for nr in 1..=grid.slices {
        if cancel() {
            return Err("cancelled".into());
        }
        let mut on = calc_slice(&p, &grid, vp, nr, threads, cancel);
        if vp.white_outside {
            on.iter_mut().for_each(|b| *b = !*b);
        }
        let f = vp.slice_file(dir, name, nr);
        std::fs::write(&f, crate::png::encode_gray1(grid.width, grid.height, &on)).map_err(|e| format!("{}: {e}", f.display()))?;
        progress(nr, grid.slices);
    }
    Ok(grid.slices)
}

/// A quick look at the voxel object: the slices of a small stack seen from
/// the front, nearer slices brighter (like MB3D's preview), `size` pixels
/// for the largest box side.  Returns RGB and the image size.
pub fn preview(sc: &Scene, vp: &VoxelParams, size: usize, threads: usize) -> Result<(Vec<u8>, usize, usize), String> {
    let size = size.clamp(16, 1024);
    let mx = vp.scale.iter().cloned().fold(1e-40, f64::max);
    let dims = vp.scale.map(|s| ((size as f64 * s / mx).round() as usize).max(2));
    let mut v = vp.clone();
    v.slices = dims[2] as u32;
    v.scale = [dims[0] as f64 / dims[2] as f64, dims[1] as f64 / dims[2] as f64, 1.0];
    let mut s = export_scene(sc, &v);
    // the DE threshold is given in voxels of the full stack
    s.de_stop = vp.de * dims[2] as f64 / vp.slices.max(2) as f64;
    v.de = s.de_stop;
    v.min_de = vp.min_de * dims[2] as f64 / vp.slices.max(2) as f64;
    let p = CalcParams::new(&s)?;
    let grid = Grid::new(&s, &v);
    let (w, h) = (grid.width, grid.height);
    let mut img = vec![[0x20u8, 0x30, 0x40]; w * h];
    let mut done = vec![false; w * h];
    let threads = if threads > 0 { threads } else { std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4) };
    let count = AtomicUsize::new(0);
    // from the front (slice 1 is nearest the camera) to the back
    for nr in 1..=grid.slices {
        let on = calc_slice(&p, &grid, &v, nr, threads, &|| false);
        let b = (255.0 - (nr as f64 / grid.slices as f64) * 200.0) as u8;
        for i in 0..w * h {
            if on[i] && !done[i] {
                done[i] = true;
                img[i] = [b, b, (b as u16 * 7 / 8) as u8];
                count.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
    Ok((img.concat(), w, h))
}

// ---------------------------------------------------------------------------
// .m3v files

/// Reads a `.m3v` project: the settings and the scene.
pub fn read_m3v(data: &[u8]) -> Result<(VoxelParams, Scene, Vec<String>), String> {
    if data.len() < M3V_SETTINGS + HEADER_SIZE {
        return Err("file too small for a MB3D voxel project".into());
    }
    let f64at = |o: usize| f64::from_le_bytes(data[o..o + 8].try_into().unwrap());
    let i32at = |o: usize| i32::from_le_bytes(data[o..o + 4].try_into().unwrap());
    let mut off = [f64at(0), f64at(8), f64at(16)];
    let mut scale = [f64at(24), f64at(32), f64at(40)];
    let slices = i32at(48).clamp(2, 1 << 20) as u32;
    let test = if i32at(52) == 0 { ObjectTest::Iterations } else { ObjectTest::De };
    let max_its = i32at(56);
    let de = f64at(64);
    let orig_width = i32at(72);
    let mut version = i32at(76);
    let mut white_outside = i32at(80) != 0;
    let default_orientation = i32at(88) != 0;
    let mut leading_zeros = i32at(92) != 0;
    let mut min_de = f64at(96);
    let mut min_its = i32at(104);
    let folder: String = data[204..204 + 1024].iter().take_while(|&&b| b != 0).map(|&b| b as char).collect();
    let mut raw = data[M3V_SETTINGS..M3V_SETTINGS + HEADER_SIZE].to_vec();
    let a = M3V_SETTINGS + HEADER_SIZE;
    raw.extend_from_slice(&data[a..data.len().min(a + ADDON_SIZE)]);
    raw.resize(HEADER_SIZE + ADDON_SIZE, 0);
    let m = crate::m3p::parse(&raw)?;
    // UpdateVersion
    if version > 100 || scale.iter().any(|&s| s < 1e-10) {
        version = 0;
    }
    if version < 2 {
        // old files stored start and end of each axis
        let dtx = (off[1] - off[0]).abs().max(1e-30);
        let dty = (scale[0] - off[2]).abs();
        let dtz = (scale[2] - scale[1]).abs();
        off = [0.0; 3];
        scale = [1.0, dty / dtx, dtz / dtx];
        leading_zeros = true;
        white_outside = false;
    }
    if version < 3 {
        let dtz = (m.scene.width - 1) as f64 * 2.2 / (64.0 * m.scene.zoom * scale[2] * (slices as f64 - 1.0));
        off = off.map(|o| o * dtz);
    }
    if version < 4 {
        min_de = de * 0.254;
        min_its /= 2;
    }
    let vp = VoxelParams {
        offset: off,
        scale,
        slices,
        test,
        max_its,
        de,
        min_its,
        min_de,
        white_outside,
        default_orientation,
        leading_zeros,
        output_folder: folder,
        orig_width,
    };
    Ok((vp, m.scene, m.warnings))
}

/// Writes a `.m3v` project (version 4).
pub fn write_m3v(vp: &VoxelParams, sc: &Scene) -> Vec<u8> {
    let mut d = Vec::with_capacity(M3V_SETTINGS + HEADER_SIZE + ADDON_SIZE);
    let f = |d: &mut Vec<u8>, v: f64| d.extend_from_slice(&v.to_le_bytes());
    let i = |d: &mut Vec<u8>, v: i32| d.extend_from_slice(&v.to_le_bytes());
    for v in vp.offset.iter().chain(&vp.scale) {
        f(&mut d, *v);
    }
    i(&mut d, vp.slices as i32);
    i(&mut d, vp.test as i32);
    i(&mut d, vp.max_its);
    i(&mut d, 0);
    f(&mut d, vp.de);
    i(&mut d, vp.orig_width);
    i(&mut d, VOXEL_VERSION);
    i(&mut d, vp.white_outside as i32);
    i(&mut d, 0); // OutputFormat: 1 bit
    i(&mut d, vp.default_orientation as i32);
    i(&mut d, vp.leading_zeros as i32);
    f(&mut d, vp.min_de.max(vp.de * 0.254));
    i(&mut d, vp.min_its);
    d.resize(204, 0);
    let folder: Vec<u8> = vp.output_folder.chars().map(|c| if (c as u32) < 256 { c as u8 } else { b'_' }).take(1023).collect();
    d.extend_from_slice(&folder);
    d.resize(M3V_SETTINGS, 0);
    let mut s = export_scene(sc, vp);
    s.light_blend = None;
    let mut raw = crate::m3p::write(&s);
    raw.resize(HEADER_SIZE + ADDON_SIZE, 0);
    d.extend_from_slice(&raw);
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bulb() -> Scene {
        let mut s = Scene::preset("Integer Power").unwrap();
        s.width = 200;
        s.height = 150;
        s
    }

    #[test]
    fn slices_cut_the_bulb() {
        let s = bulb();
        let mut vp = VoxelParams::from_scene(&s, 24);
        vp.default_orientation = true;
        // the stack covers 2.2 / zoom around the middle: the bulb (radius
        // ~1.1) fills the middle slices and not the first
        let sc = export_scene(&s, &vp);
        let p = CalcParams::new(&sc).unwrap();
        let g = Grid::new(&sc, &vp);
        let filled = |nr| calc_slice(&p, &g, &vp, nr, 2, &|| false).iter().filter(|&&b| b).count();
        let (w, h) = vp.size();
        assert_eq!((w, h), (24, 24));
        let mid = filled(12);
        assert!(mid > w * h / 3, "middle slice: {mid} of {}", w * h);
        assert!(filled(1) < mid / 4, "first slice: {}", filled(1));
        // the iteration test gives a similar object
        vp.test = ObjectTest::Iterations;
        let it = calc_slice(&p, &g, &vp, 12, 2, &|| false).iter().filter(|&&b| b).count();
        assert!((it as f64 - mid as f64).abs() < 0.25 * mid as f64, "{it} vs {mid}");
    }

    #[test]
    fn m3v_roundtrip() {
        let s = bulb();
        let mut vp = VoxelParams::from_scene(&s, 50);
        vp.offset = [0.1, -0.2, 0.05];
        vp.scale = [1.0, 0.5, 1.0];
        vp.output_folder = "C:\\voxels\\".into();
        let d = write_m3v(&vp, &s);
        assert_eq!(d.len(), M3V_SETTINGS + HEADER_SIZE + ADDON_SIZE);
        let (v2, s2, _) = read_m3v(&d).unwrap();
        assert_eq!(v2.offset, vp.offset);
        assert_eq!(v2.scale, vp.scale);
        assert_eq!(v2.slices, 50);
        assert_eq!(v2.output_folder, "C:\\voxels\\");
        assert!((v2.de - vp.de).abs() < 1e-12);
        assert_eq!(v2.size(), (50, 25));
        assert!((s2.zoom - s.zoom).abs() < 1e-12);
    }
}
