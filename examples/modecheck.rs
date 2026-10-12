//! Renders parameter files on the CPU, split between CPU and graphics card
//! (Auto's mixed mode, forced) and on the card, and compares the images:
//! `modecheck mb3d_dir max_width out_dir file...` prints one line per file
//! (tab separated: name, size, CPU s, mixed s, card's rows %, mean
//! difference %, pixels off by > 10 %, card s, mean difference %, pixels
//! off %, status) and writes cpu/mixed/card PNGs to out_dir.
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    mb3d::formulas::add_formula_dir(std::path::PathBuf::from(&a[0]).join("M3Formulas"));
    mb3d::maps::add_map_dir(std::path::PathBuf::from(&a[0]).join("M3Maps"));
    let max_w: f64 = a[1].parse().unwrap();
    let out = std::path::PathBuf::from(&a[2]);
    std::fs::create_dir_all(&out).unwrap();
    for f in &a[3..] {
        let path = std::path::Path::new(f);
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let (mut sc, _) = match mb3d::animfile::load_scene(path) {
            Ok(v) => v,
            Err(e) => {
                println!("{name}\tload error: {e}");
                continue;
            }
        };
        if sc.width as f64 > max_w {
            sc.scale_image(max_w / sc.width as f64);
        }
        // mixed: Auto with the split forced; the card: "Graphics card" (waits
        // for its shaders)
        let render = |gpu: bool, mixed: bool| {
            mb3d::gpu::set_enabled(gpu);
            mb3d::gpu::set_auto(mixed);
            if mixed {
                std::env::set_var("MB3D_GPU_MIXED", "1");
            } else {
                std::env::remove_var("MB3D_GPU_MIXED");
            }
            let t = std::time::Instant::now();
            let r = mb3d::render::render(&sc, &|_, _| {});
            (r, t.elapsed().as_secs_f64(), mb3d::gpu::last_status())
        };
        let diff = |x: &[u8], y: &[u8]| {
            let mae = x.iter().zip(y).map(|(&p, &q)| (p as f64 - q as f64).abs()).sum::<f64>() / x.len() as f64 / 2.55;
            let off = x.chunks(3).zip(y.chunks(3)).filter(|(p, q)| p.iter().zip(q.iter()).any(|(&u, &v)| (u as i32 - v as i32).abs() > 25)).count();
            (mae, off as f64 * 300.0 / x.len() as f64)
        };
        let (c, tc, _) = render(false, false);
        let c = match c {
            Ok(c) => c,
            Err(e) => {
                println!("{name}\tCPU error: {e}");
                continue;
            }
        };
        // the card's shaders compile first (not timed)
        let _ = render(true, false);
        let (m, tm, sm) = render(true, true);
        let (g, tg, sg) = render(true, false);
        let (Ok(m), Ok(g)) = (m, g) else {
            println!("{name}\tGPU error");
            continue;
        };
        let rows = sm.split(": ").nth(1).and_then(|t| t.split(" %").next()).and_then(|v| v.parse::<f64>().ok());
        let (dm, om) = diff(&c.rgb, &m.rgb);
        let (dg, og) = diff(&c.rgb, &g.rgb);
        println!(
            "{name}\t{}x{}\t{tc:.2}\t{tm:.2}\t{}\t{dm:.2}\t{om:.2}\t{tg:.2}\t{dg:.2}\t{og:.2}\t{}",
            sc.width,
            sc.height,
            rows.map_or("-".into(), |r| format!("{r:.0}")),
            if sg.starts_with("GPU") { sg.clone() } else { format!("{sg} | mixed: {sm}") }
        );
        for (k, r) in [("cpu", &c), ("mixed", &m), ("card", &g)] {
            mb3d::png::write_rgb(out.join(format!("{name}.{k}.png")).to_str().unwrap(), r.width, r.height, &r.rgb).unwrap();
        }
    }
}
