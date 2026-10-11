//! Prints the hybrid slots and the DE combination of parameter files.
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    mb3d::formulas::add_formula_dir(std::path::PathBuf::from(&a[0]).join("M3Formulas"));
    mb3d::maps::add_map_dir(std::path::PathBuf::from(&a[0]).join("M3Maps"));
    for f in &a[1..] {
        let (sc, _) = mb3d::animfile::load_scene(std::path::Path::new(f)).unwrap();
        let p = mb3d::calc::CalcParams::new(&sc).unwrap();
        println!("{f}: mode {:?} difs {} custom {} de_option {} end_to {} repeat {} max_it {} d_de_scale {}", p.mode, p.difs, p.is_custom_de, p.de_option, p.end_to, p.repeat_from, p.max_it, p.d_de_scale);
        for (i, s) in p.slots.iter().enumerate() {
            println!("  slot {i}: {} x{} uncounted {} ade {} de_option {}", s.formula.name(), s.iterations, s.uncounted, s.ade, s.formula.de_option());
        }
        println!("  decomb {:?}", p.decomb);
    }
}
