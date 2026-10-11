//! Counts the features of parameter files that decide where they render.
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    mb3d::formulas::add_formula_dir(std::path::PathBuf::from(&a[0]).join("M3Formulas"));
    mb3d::maps::add_map_dir(std::path::PathBuf::from(&a[0]).join("M3Maps"));
    let mut n: std::collections::BTreeMap<&str, usize> = Default::default();
    for f in &a[1..] {
        let Ok((sc, _)) = mb3d::animfile::load_scene(std::path::Path::new(f)) else { continue };
        let Ok(p) = mb3d::calc::CalcParams::new(&sc) else { continue };
        let name = std::path::Path::new(f).file_stem().unwrap().to_string_lossy().to_string();
        let mut tags: Vec<&str> = Vec::new();
        if sc.mc.reflections { tags.push("reflections"); }
        if sc.dof.is_some() { tags.push("dof"); }
        if sc.normals_on_zbuf { tags.push("normals_on_zbuf"); }
        if p.mode != mb3d::iteration::HybridMode::Alt3D { tags.push("4D/ipol"); }
        if p.inside_rendering || p.in_and_outside { tags.push("inside"); }
        if p.cut_options != 0 { tags.push("cut"); }
        if p.vol.is_some() || sc.vol_light.is_some() { tags.push("vol"); }
        if p.color_on_it != 0 { tags.push("color_on_it"); }
        if mb3d::gpu::precision(&p) < 3.0 { tags.push("deep zoom"); }
        let m = p.ystart.iter().fold(0.0f64, |a, v| a.max(v.abs())) + p.step_width * (p.width + p.height) as f64 + 2.0;
        let step_prec = p.step_width / (m * f32::EPSILON as f64);
        tags.push(if p.difs { "difs" } else if p.is_custom_de { "analytic" } else { "numeric" });
        let detail = format!("{}x{} precision {:.2} step/ulp {:.0}", sc.width, sc.height, mb3d::gpu::precision(&p), step_prec);
        for t in &tags { *n.entry(t).or_default() += 1; }
        let gpu = mb3d::gpu::unsupported(&p).unwrap_or_else(|| "GPU".into());
        println!("{name}: {} | {detail} | {gpu}", tags.join(", "));
    }
    println!("{n:?}");
}
