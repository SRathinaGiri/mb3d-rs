// renders a parameter file (GPU on unless CPU is set) and prints the times
// and checksums of the G-buffer and the image: gpurender mb3d_dir file [scale]
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
}
