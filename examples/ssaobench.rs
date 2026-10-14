//! Times the screen space ambient occlusion (SSAO24 or the 15 bit one) on a
//! parameter file's G-buffer, on the CPU and on the graphics card:
//! `ssaobench WIDTH FILES...` (tab separated: file, size, kind, cpu s, card s).
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let width: f64 = a[0].parse().unwrap();
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    for f in &a[1..] {
        let name = std::path::Path::new(f).file_name().unwrap().to_string_lossy().to_string();
        let Ok((mut sc, _)) = mb3d::animfile::load_scene(std::path::Path::new(f)) else { continue };
        let Some(ao) = sc.ao else { continue };
        sc.scale_image(width / sc.width.max(1) as f64);
        let Ok((p, g)) = mb3d::render::calculate_raw_cancellable(&sc, &|_, _| {}, &|| false) else { continue };
        let (w, h) = (p.rect[2] as usize, p.rect[3] as usize);
        let (zc, zcorr) = (p.zc_mul, p.zcorr);
        let time = |card: bool| {
            let mut best = f64::MAX;
            let mut sum = 0u64;
            for _ in 0..2 {
                let mut g2 = g.clone();
                let t = std::time::Instant::now();
                if card {
                    #[cfg(feature = "gpu")]
                    {
                        let r = if ao.bits15 { mb3d::gpu::ssao15(&mut g2, w, h, zc, zcorr, &ao) } else { mb3d::gpu::ssao24(&mut g2, w, h, zc, zcorr, &ao) };
                        r.unwrap();
                    }
                } else if ao.bits15 {
                    mb3d::ssao15::ssao15(&mut g2, w, h, zc, zcorr, &ao, threads);
                } else {
                    mb3d::ssao::ssao24(&mut g2, w, h, zc, zcorr, &ao, threads);
                }
                best = best.min(t.elapsed().as_secs_f64());
                sum = g2.iter().enumerate().map(|(i, s)| s.amb_shadow as u64 * (i as u64 % 977 + 1)).sum();
            }
            (best, sum)
        };
        let kind = format!("{}{}{}", if ao.bits15 { "ssao15" } else { "ssao24" }, if ao.t0 { " t0" } else { "" }, if ao.random > 0 { format!(" r{}", ao.random) } else { String::new() });
        let ((c, cs), (k, ks)) = (time(false), time(true));
        println!("{name}\t{w}x{h}\t{kind}\t{c:.3}\t{k:.3}\t{cs}\t{ks}");
    }
}
