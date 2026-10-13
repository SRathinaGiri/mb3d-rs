// compares MB3D's G-buffer (an .m3i it saved) with ours for the same
// parameters, and paints MB3D's G-buffer with our painter:
// gbcompare mb3d_dir file.m3p mb3d.m3i out_dir
// (out_dir/ours.png: our G-buffer painted, out_dir/theirs.png: MB3D's G-buffer
// painted here; depth of field is left out, as MB3D's Calculate 3D does)
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    mb3d::formulas::add_formula_dir(std::path::PathBuf::from(&a[0]).join("M3Formulas"));
    mb3d::maps::add_map_dir(std::path::PathBuf::from(&a[0]).join("M3Maps"));
    let (mut sc, _) = mb3d::animfile::load_scene(std::path::Path::new(&a[1])).unwrap();
    sc.dof = None;
    // (experiments: OFF=refl,vol,light6,... leave features out of the painting)
    let off = std::env::var("OFF").unwrap_or_default();
    let p_sc = {
        let mut s = sc.clone();
        for o in off.split(',') {
            match o {
                "refl" => s.mc.reflections = false,
                "vol" => s.vol_light = None,
                l if l.starts_with("light") => {
                    let n: usize = l[5..].parse().unwrap();
                    s.lighting.lights[n - 1].on = false;
                }
                _ => {}
            }
        }
        s
    };
    let data = std::fs::read(&a[2]).unwrap();
    let (w, h, theirs) = mb3d::m3p::gbuffer_from_m3i(&data).expect("an m3i with a G-buffer");
    assert_eq!((w, h), (sc.width as usize, sc.height as usize), "image size");
    mb3d::gpu::set_enabled(false);
    let (p, ours) = mb3d::render::calculate(&sc, &|_, _| {}).unwrap();
    let both: Vec<_> = ours.iter().zip(&theirs).filter(|(a, b)| !a.is_background() && !b.is_background()).collect();
    let n = both.len().max(1) as f64;
    let pct = |f: &dyn Fn(&mb3d::gbuffer::SiLight, &mb3d::gbuffer::SiLight) -> bool| both.iter().filter(|(a, b)| f(a, b)).count() as f64 * 100.0 / n;
    println!("object pixels in both: {} of {}; background differs: {}", both.len(), w * h, ours.iter().zip(&theirs).filter(|(a, b)| a.is_background() != b.is_background()).count());
    println!("depth off by > 256: {:.2} %", pct(&|a, b| (a.zpos_fine as i64 - b.zpos_fine as i64).abs() > 256 << 8));
    println!("normals off by > 10 degrees: {:.2} %", pct(&|a, b| {
        let d = |s: &mb3d::gbuffer::SiLight| { let v = s.normal.map(|x| x as f64); let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1.0); v.map(|x| x / l) };
        let (x, y) = (d(a), d(b));
        x[0] * y[0] + x[1] * y[1] + x[2] * y[2] < 10f64.to_radians().cos()
    }));
    println!("gradient off by > 1000: {:.2} %, otrap off by > 1000: {:.2} %", pct(&|a, b| (a.si_gradient as i32 - b.si_gradient as i32).abs() > 1000), pct(&|a, b| ((a.otrap & 0x7FFF) as i32 - (b.otrap & 0x7FFF) as i32).abs() > 1000));
    for bit in 0..6 {
        let m = 0x400u16 << bit;
        let (o, t) = (both.iter().filter(|(a, _)| a.shadow & m != 0).count(), both.iter().filter(|(_, b)| b.shadow & m != 0).count());
        if o + t > 0 {
            println!("shadow bit of light {}: ours {:.1} %, MB3D {:.1} %, differ {:.2} %", bit + 1, o as f64 * 100.0 / n, t as f64 * 100.0 / n, pct(&|a, b| (a.shadow ^ b.shadow) & m != 0));
        }
    }
    println!("low shadow bits (fog / volumetric) off by > 20: {:.2} %", pct(&|a, b| ((a.shadow & 0x3FF) as i32 - (b.shadow & 0x3FF) as i32).abs() > 20));
    let mean = |f: &dyn Fn(&mb3d::gbuffer::SiLight) -> f64, x: bool| both.iter().map(|(a, b)| f(if x { a } else { b })).sum::<f64>() / n;
    println!("ambient shadow: mean ours {:.0}, MB3D {:.0}; off by > 1000: {:.2} %", mean(&|s| s.amb_shadow as f64, true), mean(&|s| s.amb_shadow as f64, false), pct(&|a, b| (a.amb_shadow as i32 - b.amb_shadow as i32).abs() > 1000));
    let out = std::path::PathBuf::from(&a[3]);
    let save = |name: &str, g: &[mb3d::gbuffer::SiLight]| {
        let rgb = mb3d::render::paint(&p_sc, &p, g);
        mb3d::png::write_rgb(out.join(name).to_str().unwrap(), w, h, &rgb).unwrap();
    };
    save("ours.png", &ours);
    save("theirs.png", &theirs);
}
