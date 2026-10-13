// writes a parameter file as a .m3p file: txt2m3p mb3d_dir in out.m3p [width]
// (SET="key=value;..." changes scene keys first, LIGHTS_OFF="2,6" switches
// those lights off)
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    mb3d::formulas::add_formula_dir(std::path::PathBuf::from(&a[0]).join("M3Formulas"));
    let (mut sc, _) = mb3d::animfile::load_scene(std::path::Path::new(&a[1])).unwrap();
    if let Ok(set) = std::env::var("SET") {
        sc = sc.apply(&set.replace(';', "\n")).unwrap();
    }
    if let Ok(off) = std::env::var("LIGHTS_OFF") {
        for n in off.split(',').filter_map(|v| v.trim().parse::<usize>().ok()) {
            sc.lighting.lights[n - 1].on = false;
        }
    }
    if let Some(w) = a.get(3).and_then(|w| w.parse::<f64>().ok()) {
        sc.scale_image(w / sc.width as f64);
    }
    std::fs::write(&a[2], mb3d::m3p::write(&sc)).unwrap();
}
