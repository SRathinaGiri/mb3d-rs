//! The double precision probe (`render::probe_single`) on parameter files:
//! `probecheck mb3d_dir max_width file...` prints one line per file (tab
//! separated: name, size, probe's mean colour difference %, pixels
//! compared, probe ms, precision, step precision, the difference % of the
//! card's march finished on the CPU in double).  The card's single
//! precision march is compiled first (not timed).
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    mb3d::formulas::add_formula_dir(std::path::PathBuf::from(&a[0]).join("M3Formulas"));
    mb3d::maps::add_map_dir(std::path::PathBuf::from(&a[0]).join("M3Maps"));
    let max_w: f64 = a[1].parse().unwrap();
    mb3d::gpu::set_enabled(true);
    for f in &a[2..] {
        let path = std::path::Path::new(f);
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let Ok((mut sc, _)) = mb3d::animfile::load_scene(path) else {
            println!("{name}\tload error");
            continue;
        };
        if sc.width as f64 > max_w {
            sc.scale_image(max_w / sc.width as f64);
        }
        let Ok(p) = mb3d::calc::CalcParams::new(&sc) else {
            println!("{name}\tparameter error");
            continue;
        };
        if let Some(r) = mb3d::gpu::unsupported(&p) {
            println!("{name}\t-\tnot on the card: {r}");
            continue;
        }
        if !mb3d::gpu::double_possible(&p) {
            println!("{name}\t-\tdouble anyway (or no doubles)");
            continue;
        }
        let Ok(flags) = mb3d::gpu::march_prepare(&p, &Default::default()) else {
            println!("{name}\t-\tno shader");
            continue;
        };
        while !flags[0].load(std::sync::atomic::Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
        let t = std::time::Instant::now();
        let r = mb3d::render::probe_single(&sc, &p, threads);
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        match r {
            Some(f) => println!("{name}\t{}x{}\t{:.2}\t{}\t{ms:.0}\t{:.2}\t{:.1}\t{:.2}", sc.width, sc.height, f.single * 100.0, f.pixels, mb3d::gpu::precision(&p), mb3d::gpu::step_precision(&p), f.mixed * 100.0),
            None => println!("{name}\t{}x{}\t-\t0\t{ms:.0}", sc.width, sc.height),
        }
    }
}
