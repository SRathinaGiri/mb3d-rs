//! # mb3d – a Rust port of Mandelbulb3D
//!
//! The code is safe Rust: `unsafe` is allowed only where Rust has no safe
//! way (calling a C library: `oidn`; calling code compiled at run time:
//! `x86::clif_jit` and its callers), each place marked `#[allow(unsafe_code)]`
//! with the reason.  Anything else needs a discussion first.
//!
//! A port of [Mandelbulb3D](https://github.com/thargor6/mb3d)
//! (Delphi, by Jesse/"MB3D" and contributors): the core distance-estimation
//! renderer with the built-in formulas, alternating hybrids, MB3D's camera
//! model and ray marcher, normals, colouring and the basic light model.
//!
//! Module map (Rust module <- original Delphi unit):
//!
//! | module      | ported from                                         |
//! |-------------|-----------------------------------------------------|
//! | `math`      | Math3D.pas                                          |
//! | `formulas`  | formulas.pas, CustomFormulas.pas (internal formulas)|
//! | `iteration` | formulas.pas (`doHybridPas*`), TypeDefinitions.pas  |
//! | `scene`     | TMandHeader10 / HeaderTrafos.pas defaults           |
//! | `calc`      | HeaderTrafos.pas (`GetMCTparasFromHeader`), Calc.pas, CalcThread.pas |
//! | `lighting`  | HeaderTrafos.pas (`MakeLightValsFromHeaderLight`), LightAdjust.pas, PaintThread.pas |
//! | `render`    | Calc.pas (`CalcMandT`), PaintThread.pas (`PaintRows`) |
//! | `m3p`       | FileHandling.pas (`LoadParameter`): binary .m3p files |
//! | `m3p_old`   | FileHandling.pas (`ConvertHeaderFromOldParas`): parameters before MB3D 1.6 |
//! | `m3f`       | CustomFormulas.pas: .m3f custom formula files       |
//! | `x86`       | IA-32 interpreter/compiler for the formula machine code |
//! | `custom`    | memory layout + calling convention for .m3f formulas |
//! | `anim`      | Animation.pas, Interpolation.pas: keyframes and interpolation |
//! | `animfile`  | Animation.pas (`LoadAni`, save): .m3a files, .m3k text format |
//! | `batch`     | BatchForm.pas: batch rendering of parameter files |
//! | `frames`    | Animation.pas (`Timer2Timer`), Mand.pas (`DoSaveAniImage`): frame rendering, output files |

#![deny(unsafe_code)]

pub mod anim;
#[cfg(feature = "gui")]
pub mod app;
pub mod appdirs;
pub mod crash;
pub mod animfile;
pub mod batch;
pub mod calc;
pub mod checkpoint;
pub mod custom;
pub mod deao;
pub mod denoise;
#[cfg(feature = "oidn")]
// Intel Open Image Denoise is a C library: its calls are unsafe by Rust's definition
#[allow(unsafe_code)]
pub mod oidn;
pub mod dof;
pub mod formulas;
pub mod frames;
pub mod gbuffer;
pub mod gif;
#[cfg(feature = "gpu")]
pub mod gpu;
pub mod image;
pub mod iteration;
pub mod jit;
pub mod jpeg;
pub mod lighting;
pub mod m3f;
pub mod m3p;
pub mod m3p_old;
pub mod maps;
pub mod mc;
pub mod mclut;
pub mod mesh;
pub mod mutagen;
pub mod math;
pub mod png;
pub mod reflect;
pub mod render;
pub mod scene;
pub mod ssao;
pub mod ssao15;
pub mod swarm;
pub mod video;
pub mod vollight;
pub mod voxel;
pub mod x86;
#[cfg(feature = "gui")]
pub mod vcl;

pub use formulas::Formula;
pub use render::{render, RenderResult};
pub use scene::Scene;
