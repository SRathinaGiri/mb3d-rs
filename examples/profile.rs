//! Where the render time goes: for each parameter file the main
//! calculation, hard shadows, ambient occlusion and painting are timed
//! separately (CPU, all threads), as tab separated lines.
//! `cargo run --release --example profile -- WIDTH MB3D_DIR FILES...`
use std::time::Instant;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let width: f64 = a[0].parse().unwrap();
    let mb3d = std::path::PathBuf::from(&a[1]);
    mb3d::formulas::add_formula_dir(mb3d.join("M3Formulas"));
    mb3d::maps::add_map_dir(mb3d.join("M3Maps"));
    println!("file\tsize\tformulas\tmain_s\tshadow_s\tao\tao_s\tpaint_s\trefl_dof\trefl_dof_s\tgpu");
    for f in &a[2..] {
        let name = std::path::Path::new(f).file_name().unwrap().to_string_lossy().to_string();
        let mut sc = match mb3d::animfile::load_scene(std::path::Path::new(f)) {
            Ok((s, _)) => s,
            Err(e) => {
                println!("{name}\terror: {e}");
                continue;
            }
        };
        sc.scale_image(width / sc.width.max(1) as f64);
        let t = Instant::now();
        let (p, mut g) = match mb3d::render::calculate_raw_cancellable(&sc, &|_, _| {}, &|| false) {
            Ok(r) => r,
            Err(e) => {
                println!("{name}\terror: {e}");
                continue;
            }
        };
        let main_s = t.elapsed().as_secs_f64();
        // MAIN_ONLY=1: the main calculation's time and statistics (as MB3D
        // shows them: average steps per pixel, the share of object pixels)
        if std::env::var_os("MAIN_ONLY").is_some() {
            let n = g.len().max(1) as f64;
            let steps = g.iter().map(|s| (s.shadow & 0x3FF) as f64).sum::<f64>() / n;
            let hits = g.iter().filter(|s| !s.is_background()).count() as f64 * 100.0 / n;
            println!("{name}	{}x{}	main {main_s:.2} s	steps {steps:.1}	hits {hits:.1} %	formula errors {}", sc.width, sc.height, mb3d::iteration::CUSTOM_ERRORS.load(std::sync::atomic::Ordering::Relaxed));
            continue;
        }
        let formulas: Vec<String> = p
            .slots
            .iter()
            .filter(|s| s.iterations != 0)
            .map(|s| {
                let kind = match &s.formula {
                    mb3d::formulas::Formula::Custom(c) if c.def.jit.is_some() => "jit",
                    mb3d::formulas::Formula::Custom(_) => "m3f",
                    _ => "builtin",
                };
                format!("{}({kind})", s.formula.name())
            })
            .collect();
        let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
        let mut shadow_s = 0.0;
        if sc.shadows.is_some() {
            let t = Instant::now();
            mb3d::render::hard_shadows(&sc, &p, &mut g, threads);
            shadow_s = t.elapsed().as_secs_f64();
        }
        let (mut ao, mut ao_s) = ("-", 0.0);
        if let Some(a) = &sc.ao {
            ao = if sc.deao.is_some() {
                "DEAO"
            } else if a.bits15 {
                "SSAO15"
            } else {
                "SSAO24"
            };
            let mut s2 = sc.clone();
            s2.shadows = None;
            s2.normals_on_zbuf = false;
            let t = Instant::now();
            mb3d::render::post_process(&s2, &p, &mut g, threads);
            ao_s = t.elapsed().as_secs_f64();
        }
        // painting alone, then the reflections and depth of field that
        // paint() runs on top
        let mut plain = sc.clone();
        plain.mc.reflections = false;
        plain.dof = None;
        let t = Instant::now();
        let _ = mb3d::render::paint(&plain, &p, &g);
        let paint_s = t.elapsed().as_secs_f64();
        let extra = match (sc.mc.reflections, sc.dof.is_some()) {
            (true, true) => "refl+dof",
            (true, false) => "refl",
            (false, true) => "dof",
            _ => "-",
        };
        let mut extra_s = 0.0;
        if extra != "-" {
            let t = Instant::now();
            let _ = mb3d::render::paint(&sc, &p, &g);
            extra_s = (t.elapsed().as_secs_f64() - paint_s).max(0.0);
        }
        let gpu = mb3d::gpu::unsupported(&p).unwrap_or_else(|| "yes".into());
        println!(
            "{name}\t{}x{}\t{}\t{main_s:.2}\t{shadow_s:.2}\t{ao}\t{ao_s:.2}\t{paint_s:.2}\t{extra}\t{extra_s:.2}\t{gpu}",
            sc.width,
            sc.height,
            formulas.join(" + "),
        );
    }
}
