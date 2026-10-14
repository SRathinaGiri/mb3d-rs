//! Times the CPU SSAO24 on a parameter file's G-buffer: `ssaobench FILE WIDTH THREADS [REPEAT]`
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let (mut sc, _) = mb3d::animfile::load_scene(std::path::Path::new(&a[0])).unwrap();
    let width: f64 = a[1].parse().unwrap();
    let threads: usize = a[2].parse().unwrap();
    let rep: usize = a.get(3).map(|s| s.parse().unwrap()).unwrap_or(1);
    sc.scale_image(width / sc.width.max(1) as f64);
    let ao = sc.ao.unwrap();
    let (p, g) = mb3d::render::calculate_raw_cancellable(&sc, &|_, _| {}, &|| false).unwrap();
    let (w, h) = (p.rect[2] as usize, p.rect[3] as usize);
    let (zc, zcorr) = (p.zc_mul, p.zcorr);
    let mut best = f64::MAX;
    let mut sum = 0u64;
    for _ in 0..rep {
        let mut g2 = g.clone();
        let t = std::time::Instant::now();
        mb3d::ssao::ssao24(&mut g2, w, h, zc, zcorr, &ao, threads);
        best = best.min(t.elapsed().as_secs_f64());
        sum = g2.iter().map(|s| s.amb_shadow as u64).sum();
    }
    println!("{w}x{h} random {} t0 {} threads {threads}: {best:.3} s  checksum {sum}", ao.random, ao.t0);
}
