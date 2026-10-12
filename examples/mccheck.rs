// Monte Carlo passes of a parameter file with the times and a checksum of
// the records: mccheck mb3d_dir file scale passes
// SET="key=value;key=value" changes scene keys, CPU=1 keeps the GPU off,
// THREADS=n, OUT=file.png writes the painted image (its bytes to file.png.raw),
// REF=file.png prints the mean difference of the painted image to that one,
// RECS=file.txt writes the records (x y colour ysum ysqr rays hit per line)
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    mb3d::formulas::add_formula_dir(std::path::PathBuf::from(&a[0]).join("M3Formulas"));
    mb3d::maps::add_map_dir(std::path::PathBuf::from(&a[0]).join("M3Maps"));
    let (mut sc, _) = mb3d::animfile::load_scene(std::path::Path::new(&a[1])).unwrap();
    if let Ok(s) = std::env::var("SET") {
        sc = sc.apply(&s.replace(';', "\n")).unwrap();
    }
    sc.scale_image(a.get(2).map(|s| s.parse().unwrap()).unwrap_or(1.0));
    let passes: usize = a.get(3).map(|s| s.parse().unwrap()).unwrap_or(1);
    let threads: usize = std::env::var("THREADS").ok().map(|s| s.parse().unwrap()).unwrap_or(0);
    mb3d::gpu::set_enabled(std::env::var_os("CPU").is_none());
    let mut img = mb3d::mc::McImage::new(sc.width as usize, sc.height as usize);
    let hash = |b: &mut dyn Iterator<Item = u8>| b.fold(0xcbf29ce484222325u64, |h, x| (h ^ x as u64).wrapping_mul(0x100000001b3));
    let t0 = std::time::Instant::now();
    for i in 0..passes {
        let t = std::time::Instant::now();
        mb3d::mc::pass(&sc, &mut img, threads, &|_, _| {}, &|| false).unwrap();
        let h = hash(&mut img.recs.iter().flat_map(|r| {
            let mut v = Vec::with_capacity(32);
            for c in r.col {
                v.extend(c.to_bits().to_le_bytes());
            }
            v.extend(r.ysum.to_bits().to_le_bytes());
            v.extend(r.ysqr.to_bits().to_le_bytes());
            v.extend(r.ray_count.to_le_bytes());
            v.push(r.hit as u8);
            v
        }));
        let st = img.stats();
        println!("pass {}: {:.2} s, {:.1} rays/pixel, records {h:016x}", i + 1, t.elapsed().as_secs_f64(), st.avg_rays);
    }
    if let Ok(f) = std::env::var("RECS") {
        let w = sc.width as usize;
        let t: String = img.recs.iter().enumerate().map(|(i, r)| format!("{} {} {:?} {} {} {} {}\n", i % w, i / w, r.col, r.ysum, r.ysqr, r.ray_count, r.hit)).collect();
        std::fs::write(f, t).unwrap();
    }
    let rgb = mb3d::mc::paint(&img, &sc.mc, sc.lighting.gamma);
    println!("{}x{}: {:.2} s, image {:016x}  {}", sc.width, sc.height, t0.elapsed().as_secs_f64(), hash(&mut rgb.iter().copied()), mb3d::mc::last_status());
    if let Ok(f) = std::env::var("OUT") {
        mb3d::png::write_rgb(&f, sc.width as usize, sc.height as usize, &rgb).unwrap();
        std::fs::write(format!("{f}.raw"), &rgb).unwrap();
    }
    if let Ok(f) = std::env::var("REF") {
        let r = std::fs::read(format!("{f}.raw")).unwrap();
        assert_eq!(r.len(), rgb.len());
        let mae = rgb.iter().zip(&r).map(|(&x, &y)| (x as f64 - y as f64).abs()).sum::<f64>() / rgb.len() as f64 / 2.55;
        let bias = rgb.iter().zip(&r).map(|(&x, &y)| x as f64 - y as f64).sum::<f64>() / rgb.len() as f64 / 2.55;
        println!("against {f}: mean difference {mae:.2} %, mean bias {bias:+.3} %");
    }
}
