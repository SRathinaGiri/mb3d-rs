// renders a parameter file (GPU on unless CPU is set) and prints the times
// and checksums of the G-buffer and the image: gpurender mb3d_dir file [scale]
// COMPARE=dir: renders on the CPU too, prints the mean difference of the
// images and writes both (gpu.png, cpu.png) to dir
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    mb3d::formulas::add_formula_dir(std::path::PathBuf::from(&a[0]).join("M3Formulas"));
    mb3d::maps::add_map_dir(std::path::PathBuf::from(&a[0]).join("M3Maps"));
    let (mut sc, _) = mb3d::animfile::load_scene(std::path::Path::new(&a[1])).unwrap();
    if let Some(s) = a.get(2) {
        sc.scale_image(s.parse().unwrap());
    }
    mb3d::gpu::set_enabled(std::env::var_os("CPU").is_none());
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
        let dir = std::path::PathBuf::from(dir);
        let _ = std::fs::create_dir_all(&dir);
        mb3d::png::write_rgb(dir.join("gpu.png").to_str().unwrap(), r.width, r.height, &r.rgb).unwrap();
        mb3d::png::write_rgb(dir.join("cpu.png").to_str().unwrap(), c.width, c.height, &c.rgb).unwrap();
    }
}
