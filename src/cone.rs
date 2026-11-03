//! Cone march (experimental, not in MB3D; `Experimental::cone`): the
//! marches of a block of pixels started where the block's coarse ray
//! stopped.
//!
//! The coarse ray goes from the middle of the block.  At depth `t` (in
//! step widths, as the march counts) every pixel's ray of the block lies
//! within `a + slope * t` of it: `a` the widest gap of the pixels' starts
//! from its start, `slope` the widest gap of their directions (per step
//! width).  The distance estimate at the coarse ray's point is a bound on
//! the distance of the surface; the pixels' marches find the surface where
//! the estimate falls under the DE stop, so the coarse ray keeps the DE
//! stop (which grows with the depth) in its reach too: it steps while
//! `de - (a + slope * t + de_stop(t))` is positive, with the step factor of
//! the march (`z_step_div`) and its slowing down when the estimate falls
//! faster than the steps (`rsf_mul`).  Where it stops, every pixel's ray
//! is known to have an estimate above the DE stop up to there (within the
//! march's own assumption that the estimate is a true bound), so the
//! pixel's march goes there in one step ([`Marcher::march_pixel`]).  A
//! coarse ray that passes the end of the view makes its block background.
//!
//! Two levels: blocks of [`LEVELS`] pixels, the coarser first; a finer
//! block's coarse ray starts where its coarser block's stopped.  The blocks
//! are calculated when a pixel of them is first marched, by that thread;
//! another thread wanting the same block waits for it.
//!
//! The dynamic fog counts the steps of the march: a pixel's march starts
//! with the count the coarse ray estimated for the way to its depth (the
//! march's steps per length along the coarse ray).  Not with the
//! volumetric light (it integrates along every ray), "disable analytic
//! DE" (its estimate is no bound), inside and outside together (the start
//! decides the side per pixel) and 2D slices.

use crate::calc::{CalcParams, Marcher};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// The block sizes of the levels in pixels, the coarsest first.
pub const LEVELS: [usize; 2] = [16, 4];

/// a block not yet calculated / being calculated (no depth has these bits:
/// depths are finite non-negative or +inf)
const EMPTY: u64 = u64::MAX;
const BUSY: u64 = u64::MAX - 1;

#[derive(Debug)]
struct Level {
    b: usize,
    /// the first block column and row (the rectangle's)
    bx0: usize,
    by0: usize,
    cols: usize,
    /// the depth of each block (f64 bits), row by row
    cells: Vec<AtomicU64>,
    /// the dynamic fog's step count up to the depth (f32 bits; written
    /// before the depth)
    counts: Vec<AtomicU32>,
}

/// The coarse rays' depths of a calculation's rectangle, per level.
#[derive(Debug)]
pub struct Grid {
    levels: Vec<Level>,
}

impl Grid {
    /// For the pixels of `rect` (x, y, width, height of the picture).
    pub fn new(rect: [i32; 4]) -> Grid {
        let [x0, y0, w, h] = rect.map(|v| v.max(0) as usize);
        let levels = LEVELS
            .iter()
            .map(|&b| {
                let (bx0, by0) = (x0 / b, y0 / b);
                let cols = (x0 + w).div_ceil(b) - bx0;
                let rows = (y0 + h).div_ceil(b) - by0;
                Level { b, bx0, by0, cols, cells: (0..cols * rows).map(|_| AtomicU64::new(EMPTY)).collect(), counts: (0..cols * rows).map(|_| AtomicU32::new(0)).collect() }
            })
            .collect();
        Grid { levels }
    }

    /// The depth up to which the ray of pixel (x, y) is free of the surface
    /// (+inf: background) and the dynamic fog's step count up to there,
    /// calculated with `m` when its block is new.
    pub fn safe_depth(&self, m: &mut Marcher, x: i32, y: i32) -> (f64, f32) {
        self.depth(self.levels.len() - 1, m, x, y)
    }

    fn depth(&self, li: usize, m: &mut Marcher, x: i32, y: i32) -> (f64, f32) {
        let l = &self.levels[li];
        let (bx, by) = (x as usize / l.b, y as usize / l.b);
        let i = (by - l.by0) * l.cols + (bx - l.bx0);
        let Some(cell) = l.cells.get(i) else { return (0.0, 0.0) };
        match cell.compare_exchange(EMPTY, BUSY, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => {
                let (t0, c0) = if li == 0 { (0.0, 0.0) } else { self.depth(li - 1, m, x, y) };
                let (t, c) = if t0.is_infinite() { (t0, c0) } else { m.cone_march(bx, by, l.b, t0, c0) };
                l.counts[i].store(c.to_bits(), Ordering::Relaxed);
                cell.store(t.to_bits(), Ordering::Release);
                (t, c)
            }
            Err(mut v) => {
                while v == BUSY {
                    std::thread::yield_now();
                    v = cell.load(Ordering::Acquire);
                }
                (f64::from_bits(v), f32::from_bits(l.counts[i].load(Ordering::Relaxed)))
            }
        }
    }
}

static PIXELS: AtomicU64 = AtomicU64::new(0);
static FROM_START: AtomicU64 = AtomicU64::new(0);

/// A pixel marched with the cone march; `from_start`: its jump landed in
/// the surface (the estimate was no bound there), it was marched from the
/// start.  (Counted for the stage times.)
pub fn note_jump(from_start: bool) {
    PIXELS.fetch_add(1, Ordering::Relaxed);
    if from_start {
        FROM_START.fetch_add(1, Ordering::Relaxed);
    }
}

/// The pixels marched with the cone march so far (on the CPU) and those
/// marched from the start.
pub fn counts() -> (u64, u64) {
    (PIXELS.load(Ordering::Relaxed), FROM_START.load(Ordering::Relaxed))
}

/// Why the scene's calculation does not use the cone march (None: it does).
pub fn not_wanted(sc: &crate::scene::Scene) -> Option<&'static str> {
    if !sc.experimental.cone() {
        Some("not switched on")
    } else if sc.vol_light.is_some() {
        Some("volumetric light")
    } else if sc.disable_analytic_de {
        Some("disable analytic DE")
    } else if sc.inside == crate::scene::InsideMode::Both {
        Some("inside and outside")
    } else if sc.slice_2d != 0 {
        Some("2D slice")
    } else {
        None
    }
}

/// The grid of a calculation's parameters when the scene wants it.
pub fn grid_for(sc: &crate::scene::Scene, p: &CalcParams) -> Option<std::sync::Arc<Grid>> {
    let why = not_wanted(sc);
    if sc.experimental.cone() && std::env::var_os("MB3D_STAGE_TIMES").is_some() {
        crate::elog!("  cone march: {}", why.unwrap_or("on"));
    }
    why.is_none().then(|| std::sync::Arc::new(Grid::new(p.rect)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::{CalcParams, Marcher};
    use crate::scene::{CameraOptic, Scene};

    fn scene(extra: &str) -> Scene {
        let mut sc = Scene::preset("Integer Power").unwrap();
        sc.width = 96;
        sc.height = 64;
        sc.apply(&format!("experimental = on\ncone_march = on\nfirst_step_random = off\n{extra}")).unwrap()
    }

    /// The depth of a pixel's surface from its G-buffer entry (step widths).
    fn depth_of(p: &CalcParams, si: &crate::gbuffer::SiLight) -> f64 {
        let zf = (si.zpos_fine >> 8) as f64;
        (((8388351.5 - zf) / p.zc_mul + 1.0).powi(2) - 1.0) / p.zcorr
    }

    #[test]
    fn the_blocks_depths_lie_before_the_pixels_surfaces() {
        let sc = scene("");
        let p = CalcParams::new(&sc).unwrap();
        let g = p.cone.clone().expect("the grid");
        let plain = CalcParams { cone: None, ..p.clone() };
        let (mut m, mut mp) = (Marcher::new(&p, 0), Marcher::new(&plain, 0));
        let (mut jumps, mut behind, mut bg_blocks, mut edge) = (0, 0, 0, 0usize);
        for y in 0..sc.height {
            for x in 0..sc.width {
                let (safe, _) = g.safe_depth(&mut m, x, y);
                let (t16, _) = g.depth(0, &mut m, x, y);
                assert!(safe >= t16, "the finer block starts where the coarser stopped");
                let si = mp.march_pixel(x, y);
                if safe.is_infinite() {
                    bg_blocks += 1;
                    assert!(si.zpos() >= 32768, "({x}, {y}): the block's ray passed the end, the pixel is background");
                } else if safe > 0.0 {
                    jumps += 1;
                    if si.zpos() < 32768 && depth_of(&p, &si) < safe {
                        behind += 1;
                    }
                }
                // the pixel with the cone march: the same kind of result
                // (but for a few at the edge, where a ray grazes the surface)
                let sc_ = m.march_pixel(x, y);
                edge += ((sc_.zpos() >= 32768) != (si.zpos() >= 32768)) as usize;
            }
        }
        assert!(bg_blocks > 100 && jumps > 1000, "{bg_blocks} background pixels by their block, {jumps} jumps");
        // (the Mandelbulb's estimate is no strict bound: a few)
        assert!(behind * 200 < jumps, "{behind} of {jumps} jumps passed the surface");
        assert!(edge * 100 < (sc.width * sc.height) as usize, "{edge} pixels background on one side only");
    }

    #[test]
    fn the_dynamic_fogs_count_comes_with_the_depth() {
        let sc = scene("dyn_fog = 60");
        let p = CalcParams::new(&sc).unwrap();
        let g = p.cone.clone().unwrap();
        let mut m = Marcher::new(&p, 0);
        let (safe, count) = g.safe_depth(&mut m, 0, 0);
        assert!(safe.is_infinite() && count > 0.0, "the corner is background with the fog's count of the way: {safe} {count}");
        let si = m.march_pixel(0, 0);
        assert!(si.zpos() >= 32768 && si.shadow > 0, "{} {}", si.zpos(), si.shadow);
    }

    /// LightBulbMoon - No, Never (Amazing Surf, 1000 iterations), 192 by 192:
    /// its estimate is far too large off the surface, only the march's
    /// step limit finds the thin parts; without that limit the coarse rays
    /// passed them (at margin 3 a 55th of the surface became background,
    /// with the limit a 200th, the march's edge pixels).
    #[test]
    fn the_coarse_rays_keep_the_marchs_step_limit() {
        // _Rotate and Amazing Surf are .m3f files: from the MB3D repository
        // (MB3D_REPO, as in CI) or the usual formula folders, else skipped
        if let Ok(repo) = std::env::var("MB3D_REPO") {
            crate::formulas::add_formula_dir(std::path::PathBuf::from(repo).join("M3Formulas"));
        }
        if let Err(e) = crate::formulas::load_custom("_Rotate").and(crate::formulas::load_custom("Amazing Surf")) {
            eprintln!("skipped: {e}");
            return;
        }
        let sc = Scene::parse(
            "width = 192\nheight = 192\niterations = 1000\nmin_iterations = 1\nrstop = 1024\n\
             zoom = 39.7619219387428\nmid = 0.590668486717442, -4.80312498339983, 0.552945843153886\n\
             z_start = 0.439069097449697\nz_end = 128.821828929992\n\
             vgrads = -0.00002291690465552792, -0.000005663780594645785, 0.000012774063303579667, 0.0000019315813555699398, 0.000023017705602866107, 0.000013670912374282298, -0.000013839223149258143, 0.00001259152258681232, -0.00001924497556427487\n\
             fov = 60\nde_stop = 0.384\nrepeat_from = 1\nraystep = 0.3\nraystep_limiter = 0.3\nbin_search = 11\n\
             step_sub_de_stop = true\nvary_de_stop = true\nfirst_step_random = false\n\
             julia = true\njulia_c = 2.38125, 0.0109375, 0.0498046875, 0\n\
             experimental = on\ncone_march = on\n\
             [formula]\nname = _Rotate\niterations = 1\nrotation1 x = 25.1484375\nrotation1 y = -2.3671875\nrotation1 z = 64.21875\n\
             [formula]\nname = Amazing Surf\niterations = 1\nscale = 1.0444335937499998\nmin r = -2.05625\nfold = 1.0703125\n\
             rotation1 x = 5.1640625\nrotation1 y = 5\nrotation1 z = 2.609375\nscale vary = 0\nsphere or cylinder = 0\n",
        )
        .unwrap();
        let p = CalcParams::new(&sc).unwrap();
        assert!(p.cone.is_some());
        let plain = CalcParams { cone: None, ..p.clone() };
        let (mut m, mut mp) = (Marcher::new(&p, 0), Marcher::new(&plain, 0));
        let (mut surface, mut lost) = (0, 0);
        for y in 0..sc.height {
            for x in 0..sc.width {
                let hit = mp.march_pixel(x, y).zpos() < 32768;
                surface += hit as usize;
                lost += (hit && m.march_pixel(x, y).zpos() >= 32768) as usize;
            }
        }
        assert!(surface > 10000, "{surface} pixels on the surface");
        assert!(lost * 100 < surface, "{lost} of {surface} surface pixels background with the cone march");
    }

    #[test]
    fn when_the_cone_march_is_not_used() {
        assert_eq!(not_wanted(&scene("")), None);
        assert_eq!(not_wanted(&scene("experimental = off")), Some("not switched on"));
        assert_eq!(not_wanted(&scene("cone_march = off")), Some("not switched on"));
        assert_eq!(not_wanted(&scene("disable_analytic_de = true")), Some("disable analytic DE"));
        assert_eq!(not_wanted(&scene("inside = both")), Some("inside and outside"));
        assert!(CalcParams::new(&scene("inside = both")).unwrap().cone.is_none());
    }

    #[test]
    fn the_ray_of_a_pixel_is_the_marchs() {
        for optic in [CameraOptic::Common, CameraOptic::Planar, CameraOptic::Fisheye, CameraOptic::Panorama] {
            let mut sc = Scene::preset("Integer Power").unwrap();
            sc.width = 64;
            sc.height = 48;
            sc.optic = optic;
            let p = CalcParams::new(&sc).unwrap();
            let mut m = Marcher::new(&p, 0);
            for (x, y) in [(0, 0), (17, 5), (63, 47)] {
                let (c, v) = m.ray(x as f64, y as f64);
                m.cafy = (y as f64 / p.height as f64 - 0.5) * p.fov_y;
                m.calc_vgrads_fov(x + 1);
                m.calc_start_pos(x, y);
                for k in 0..3 {
                    assert!((c[k] - m.it.c[k]).abs() < 1e-9 * (1.0 + c[k].abs()), "{optic:?} ({x}, {y}) start {c:?} {:?}", m.it.c);
                    assert!((v[k] - m.vfov[k]).abs() < 1e-6 * (1.0 + v[k].abs()), "{optic:?} ({x}, {y}) direction {v:?} {:?}", m.vfov);
                }
            }
        }
    }
}
