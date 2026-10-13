//! Times the painting stages alone (painting, reflections, depth of field;
//! CPU, all threads) of one parameter file, with the G-buffer of the main
//! calculation and ambient occlusion cached in an `.m3i`, so that a change
//! of the painter can be measured without the calculation.
//! `cargo run --release --example paintbench -- MB3D_DIR FILE CACHE.m3i [REPEATS]`
use std::time::Instant;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let mb3d = std::path::PathBuf::from(&a[0]);
    mb3d::formulas::add_formula_dir(mb3d.join("M3Formulas"));
    mb3d::maps::add_map_dir(mb3d.join("M3Maps"));
    let repeats: usize = a.get(3).map_or(1, |s| s.parse().unwrap());
    let (sc, _) = mb3d::animfile::load_scene(std::path::Path::new(&a[1])).unwrap();
    mb3d::gpu::set_enabled(false);
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let cache = std::path::Path::new(&a[2]);
    let (p, g) = match std::fs::read(cache).ok().and_then(|d| mb3d::m3p::gbuffer_from_m3i(&d)) {
        Some((w, h, g)) if (w, h) == (sc.width as usize, sc.height as usize) => {
            println!("G-buffer from {}", cache.display());
            (mb3d::calc::CalcParams::new(&sc).unwrap(), g)
        }
        _ => {
            let t = Instant::now();
            let (p, mut g) = mb3d::render::calculate(&sc, &|_, _| {}).unwrap();
            println!("main calculation {:.2} s", t.elapsed().as_secs_f64());
            let t = Instant::now();
            mb3d::render::post_process(&sc, &p, &mut g, threads);
            println!("post process {:.2} s", t.elapsed().as_secs_f64());
            std::fs::write(cache, mb3d::m3p::write_m3i(&sc, &g)).unwrap();
            (p, g)
        }
    };
    let mut plain = sc.clone();
    plain.mc.reflections = false;
    plain.dof = None;
    let mut refl = sc.clone();
    refl.dof = None;
    for _ in 0..repeats {
        let t = Instant::now();
        let _ = mb3d::render::paint(&plain, &p, &g);
        let paint_s = t.elapsed().as_secs_f64();
        let t = Instant::now();
        let _ = mb3d::render::paint(&refl, &p, &g);
        let refl_s = (t.elapsed().as_secs_f64() - paint_s).max(0.0);
        let t = Instant::now();
        let rgb = mb3d::render::paint(&sc, &p, &g);
        let dof_s = (t.elapsed().as_secs_f64() - paint_s - refl_s).max(0.0);
        println!("paint {paint_s:.2} s  reflections {refl_s:.2} s  dof {dof_s:.2} s");
        if let Some(out) = std::env::var_os("OUT") {
            mb3d::png::write_rgb(out.to_str().unwrap(), sc.width as usize, sc.height as usize, &rgb).unwrap();
        }
    }
}
