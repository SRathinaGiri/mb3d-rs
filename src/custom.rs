//! Running `.m3f` custom formulas inside the iteration loop.
//!
//! Memory layout of the emulated 32-bit address space (see `x86.rs`):
//!
//! | address            | content                                      |
//! |--------------------|----------------------------------------------|
//! | BASE + 0x0000      | header for the native test oracle            |
//! | BASE + 0x1000      | `TIteration3Dext` (J4 at +0, C1 at +56)      |
//! | BASE + 0x2000 + n*0x400 | variable buffer of hybrid slot n (pConst at +256) |
//! | BASE + 0x4000..0x7F00 | stack                                     |
//! | BASE + 0x8000 + n*0x1000 | machine code of slot n                 |
//! | BASE + 0xF000      | host functions (internal integer powers)     |

use crate::m3f::{M3f, CONST_OFFSET};
use crate::x86::{EmuError, Machine, BASE, MAGIC_START};
use std::sync::Arc;

pub const IT_BASE: u32 = BASE + 0x1000;
/// `PIteration3D` (address of C1)
pub const IT_C1: u32 = IT_BASE + 56;
pub const STACK_TOP: u32 = BASE + 0x7F00;
pub const MAX_STEPS: u64 = 2_000_000;
/// The machine's cap once a formula exceeded [`MAX_STEPS`]: a loop of its
/// that never ends with these options (MB3D hangs in it; e.g. an integer
/// power of 0 in TrifoxComplexAngles), so the later calls of this render
/// fail at their first step instead of each running the whole cap
/// (a 40 x 32 probe of such a scene took minutes).
pub const STEPS_AFTER_LOOP: u64 = 0;

pub fn var_buf_addr(slot: usize) -> u32 {
    BASE + 0x2000 + slot as u32 * 0x400
}
pub fn pconst_addr(slot: usize) -> u32 {
    var_buf_addr(slot) + CONST_OFFSET as u32
}
pub fn code_addr(slot: usize) -> u32 {
    BASE + 0x8000 + slot as u32 * 0x1000
}
/// Host address of the internal integer power function n (2..8).
pub fn int_pow_addr(n: i32) -> u32 {
    MAGIC_START + (n.clamp(2, 8) as u32) * 16
}

/// Offsets inside TIteration3Dext, relative to J4 (IT_BASE).
pub mod off {
    pub const J4: u32 = 0;
    pub const ROLD: u32 = 8;
    pub const RSTOPD: u32 = 16;
    pub const X: u32 = 24;
    pub const C1: u32 = 56;
    pub const J1: u32 = 80;
    pub const PVAR: u32 = 104;
    pub const SMOOTH_IT: u32 = 108;
    pub const ROUT: u32 = 112;
    pub const IT_RESULT: u32 = 120;
    pub const MAX_IT: u32 = 124;
    pub const RSTOP: u32 = 128;
    pub const N_HYBRID: u32 = 132;
    pub const FHPVAR: u32 = 156;
    pub const CALC_SIT: u32 = 204;
    pub const END_TO: u32 = 206;
    pub const DO_JULIA: u32 = 208;
    pub const LN_RSTOP: u32 = 212;
    pub const DE_OPTION: u32 = 216;
    pub const FHLN: u32 = 220;
    pub const REPEAT_FROM: u32 = 244;
    pub const START_FROM: u32 = 246;
    pub const OTRAP: u32 = 248;
    pub const VARY_SCALE: u32 = 256;
    pub const FIRST_IT: u32 = 264;
    pub const BTMP: u32 = 268;
    pub const DFREE1: u32 = 272;
    pub const DFREE2: u32 = 280;
    pub const DERIV1: u32 = 288;
    pub const SMATRIX4: u32 = 312;
    pub const JU1: u32 = 376;
    pub const PMAPFUNC: u32 = 408;
    pub const INSIDE_RENDER: u32 = 440;
}

/// A loaded custom formula with its option values for one hybrid slot.
#[derive(Clone, Debug)]
pub struct CustomFormula {
    pub def: Arc<M3f>,
    pub values: Vec<f64>,
}

impl CustomFormula {
    pub fn new(def: Arc<M3f>) -> Self {
        let values = def.defaults();
        CustomFormula { def, values }
    }
    pub fn name(&self) -> &str {
        &self.def.name
    }
}

impl PartialEq for CustomFormula {
    fn eq(&self, o: &Self) -> bool {
        self.def.name == o.def.name && self.values == o.values
    }
}

/// Host implementation of MB3D's internal integer power functions
/// (`fHIntFunctions`), callable from formula code (FOLDING option type).
fn host_int_pow(m: &mut Machine, n: i32) -> Result<(), EmuError> {
    use crate::formulas::Formula;
    // register convention: eax=@x edx=@y ecx=@z, [esp+4]=PIt, [esp+8]=@w
    let (px, py, pz) = (m.reg(0), m.reg(2), m.reg(1));
    let pit = m.stack_arg(0)?;
    let pvar = m.rd32(pit + 48)?;
    let zmul = m.rdf64(pvar - 16)?;
    let mut v = [m.rdf64(px)?, m.rdf64(py)?, m.rdf64(pz)?, 0.0];
    let mut j = [m.rdf64(pit + 24)?, m.rdf64(pit + 32)?, m.rdf64(pit + 40)?, 0.0];
    Formula::IntPow { power: n, z_mul: zmul }.iterate(&mut v, &mut j, 0.0, false);
    m.wrf64(px, v[0])?;
    m.wrf64(py, v[1])?;
    m.wrf64(pz, v[2])?;
    m.drop_args(8)
}

/// Host addresses of the map functions (`PMapFunc`, `PMapFunc2`).
pub const MAP_SPHERE_ADDR: u32 = MAGIC_START + 0x200;
pub const MAP_XY_ADDR: u32 = MAGIC_START + 0x210;

/// `GetMapPixelSphereSpline` (`sphere`) / `GetMapPixelDirectXYspline`:
/// the colour (0..1) of map `nr` at the direction `v` / at x, y.
pub fn map_px(sphere: bool, nr: i32, v: [f64; 3]) -> [f64; 3] {
    crate::maps::with_map(nr, |lm| map_px_of(lm, sphere, v)).unwrap_or([0.0; 3])
}

fn map_px_of(lm: &crate::maps::LightMap, sphere: bool, v: [f64; 3]) -> [f64; 3] {
    let (w, h) = (lm.width as f64, lm.height as f64);
    if sphere {
        let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-300);
        let d = [v[0] / l, v[1] / l, v[2] / l];
        // MPi05d = -0.5 / Pi
        let x = d[0].atan2(d[1]) * (-0.5 / std::f64::consts::PI) + 0.5;
        let y = 0.5 - d[2].clamp(-1.0, 1.0).asin() / std::f64::consts::PI;
        lm.spline_px((x * w).clamp(0.0, w), (y * h).clamp(0.0, h))
    } else {
        let fa = |d: f64| d - d.floor();
        lm.spline_px(fa(v[0]) * w, fa(v[1]) * h)
    }
}

/// The map functions called from formula code: eax = @vec (3 doubles),
/// edx = map number, ecx = @result.
fn host_map(m: &mut Machine, sphere: bool) -> Result<(), EmuError> {
    let (pv, nr, pr) = (m.reg(0), m.reg(2) as i32, m.reg(1));
    let v = [m.rdf64(pv)?, m.rdf64(pv + 8)?, m.rdf64(pv + 16)?];
    let r = map_px(sphere, nr, v);
    m.wrf64(pr, r[0])?;
    m.wrf64(pr + 8, r[1])?;
    m.wrf64(pr + 16, r[2])
}

fn host_map_sphere(m: &mut Machine) -> Result<(), EmuError> {
    host_map(m, true)
}
fn host_map_xy(m: &mut Machine) -> Result<(), EmuError> {
    host_map(m, false)
}

macro_rules! ipow {
    ($n:expr) => {{
        fn f(m: &mut Machine) -> Result<(), EmuError> {
            host_int_pow(m, $n)
        }
        f as crate::x86::HostFn
    }};
}

/// Creates a machine with the formulas' code and variable buffers loaded.
pub fn build_machine(slots: &[Option<&CustomFormula>]) -> Machine {
    let mut m = Machine::new();
    m.host = vec![
        (int_pow_addr(2), ipow!(2)),
        (int_pow_addr(3), ipow!(3)),
        (int_pow_addr(4), ipow!(4)),
        (int_pow_addr(5), ipow!(5)),
        (int_pow_addr(6), ipow!(6)),
        (int_pow_addr(7), ipow!(7)),
        (int_pow_addr(8), ipow!(8)),
        (MAP_SPHERE_ADDR, host_map_sphere),
        (MAP_XY_ADDR, host_map_xy),
    ];
    for (i, s) in slots.iter().enumerate() {
        if let Some(cf) = s {
            let buf = cf.def.var_buffer(&cf.values, &int_pow_addr);
            m.write_bytes(var_buf_addr(i), &buf).unwrap();
            m.write_bytes(code_addr(i), &cf.def.code).unwrap();
        }
    }
    // int3 in the host area (the native oracle traps there)
    let host = vec![0xCCu8; 0x1000];
    m.write_bytes(MAGIC_START, &host).unwrap();
    m.write_bytes(IT_BASE + off::PMAPFUNC, &MAP_SPHERE_ADDR.to_le_bytes()).unwrap();
    m.write_bytes(IT_BASE + off::PMAPFUNC + 4, &MAP_XY_ADDR.to_le_bytes()).unwrap();
    let entries: Vec<u32> = (0..slots.len()).filter(|&i| slots[i].is_some()).map(code_addr).collect();
    m.predecode(&entries);
    m
}
