// renders a parameter file (GPU on unless CPU is set) and prints the times
// and checksums of the G-buffer and the image: gpurender mb3d_dir file [scale]
// COMPARE=dir: renders on the CPU too, prints the mean difference of the
// images and writes both (gpu.png, cpu.png) to dir; MAXW=n: at most n
// pixels wide; NOAUTO: the card waits for its shaders (no mixed start)
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    mb3d::formulas::add_formula_dir(std::path::PathBuf::from(&a[0]).join("M3Formulas"));
    mb3d::maps::add_map_dir(std::path::PathBuf::from(&a[0]).join("M3Maps"));
    let (mut sc, _) = mb3d::animfile::load_scene(std::path::Path::new(&a[1])).unwrap();
    if let Some(s) = a.get(2) {
        sc.scale_image(s.parse().unwrap());
    }
    // MAXW=n: at most n pixels wide
    if let Some(w) = std::env::var("MAXW").ok().and_then(|v| v.parse::<f64>().ok()) {
        if sc.width as f64 > w {
            sc.scale_image(w / sc.width as f64);
        }
    }
    mb3d::gpu::set_enabled(std::env::var_os("CPU").is_none());
    // NOAUTO: "Calculate on: Graphics card" (waits for the card's shaders)
    if std::env::var_os("NOAUTO").is_some() {
        mb3d::gpu::set_auto(false);
    }
    let t = std::time::Instant::now();
    let r = mb3d::render::render(&sc, &|_, _| {}).unwrap();
    let hash = |b: &mut dyn Iterator<Item = u8>| b.fold(0xcbf29ce484222325u64, |h, x| (h ^ x as u64).wrapping_mul(0x100000001b3));
    let g = hash(&mut r.gbuffer.iter().flat_map(|s| format!("{:?}", s).into_bytes()));
    let i = hash(&mut r.rgb.iter().copied());
    println!("{}x{}: {:.3}s (calc {:.2} s, paint {:.2} s)  gbuffer {g:016x} image {i:016x}  {}", sc.width, sc.height, t.elapsed().as_secs_f64(), r.calc_seconds, r.paint_seconds, mb3d::gpu::last_status());
    if let Some(dir) = std::env::var_os("COMPARE") {
        mb3d::gpu::set_enabled(false);
        let t = std::time::Instant::now();
        let c = mb3d::render::render(&sc, &|_, _| {}).unwrap();
        let mae = r.rgb.iter().zip(&c.rgb).map(|(&x, &y)| (x as f64 - y as f64).abs()).sum::<f64>() / r.rgb.len() as f64 / 2.55;
        let off = r.rgb.chunks(3).zip(c.rgb.chunks(3)).filter(|(x, y)| x.iter().zip(y.iter()).any(|(&p, &q)| (p as i32 - q as i32).abs() > 25)).count();
        println!("cpu {:.3}s: mean difference {mae:.2} %, {:.2} % of the pixels differ by more than 10 %", t.elapsed().as_secs_f64(), off as f64 * 100.0 / (r.width * r.height) as f64);
        // which G-buffer values differ (object pixels in both)
        let both: Vec<_> = r.gbuffer.iter().zip(&c.gbuffer).filter(|(a, b)| !a.is_background() && !b.is_background()).collect();
        let n = both.len().max(1) as f64;
        let pct = |f: &dyn Fn(&mb3d::gbuffer::SiLight, &mb3d::gbuffer::SiLight) -> bool| both.iter().filter(|(a, b)| f(a, b)).count() as f64 * 100.0 / n;
        println!(
            "G-buffer: otrap bit 15 {:.2} %, otrap off by > 1000 {:.2} %, gradient off by > 1000 {:.2} %, gradient bit 15 {:.2} %, depth off by > 256 {:.2} %",
            pct(&|a, b| (a.otrap ^ b.otrap) & 0x8000 != 0),
            pct(&|a, b| ((a.otrap & 0x7FFF) as i32 - (b.otrap & 0x7FFF) as i32).abs() > 1000),
            pct(&|a, b| ((a.si_gradient & 0x7FFF) as i32 - (b.si_gradient & 0x7FFF) as i32).abs() > 1000),
            pct(&|a, b| (a.si_gradient ^ b.si_gradient) & 0x8000 != 0),
            pct(&|a, b| (a.zpos_fine as i64 - b.zpos_fine as i64).abs() > 256 << 8),
        );
        println!(
            "          roughness off by > 20 {:.2} % (mean GPU {:.1}, CPU {:.1})",
            pct(&|a, b| ((a.zpos_fine & 0xFF) as i32 - (b.zpos_fine & 0xFF) as i32).abs() > 20),
            both.iter().map(|(a, _)| (a.zpos_fine & 0xFF) as f64).sum::<f64>() / n,
            both.iter().map(|(_, b)| (b.zpos_fine & 0xFF) as f64).sum::<f64>() / n,
        );
        println!(
            "          shadow bits {:.2} %, step count off by > 50 {:.2} %, ambient off by > 1000 {:.2} %, normals off by > 25 degrees {:.2} %",
            pct(&|a, b| (a.shadow ^ b.shadow) & 0xFC00 != 0),
            pct(&|a, b| ((a.shadow & 0x3FF) as i32 - (b.shadow & 0x3FF) as i32).abs() > 50),
            pct(&|a, b| (a.amb_shadow as i32 - b.amb_shadow as i32).abs() > 1000),
            pct(&|a, b| {
                let (p, q) = (a.normal.map(|v| v as f64), b.normal.map(|v| v as f64));
                (p[0] * q[0] + p[1] * q[1] + p[2] * q[2]) / ((p[0] * p[0] + p[1] * p[1] + p[2] * p[2]) * (q[0] * q[0] + q[1] * q[1] + q[2] * q[2])).sqrt().max(1.0) < 0.9
            }),
        );
        let dir = std::path::PathBuf::from(dir);
        let _ = std::fs::create_dir_all(&dir);
        mb3d::png::write_rgb(dir.join("gpu.png").to_str().unwrap(), r.width, r.height, &r.rgb).unwrap();
        mb3d::png::write_rgb(dir.join("cpu.png").to_str().unwrap(), c.width, c.height, &c.rgb).unwrap();
    }
}
