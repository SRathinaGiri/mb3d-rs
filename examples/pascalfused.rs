//! The compiled hybrid loop with Pascal formulas against the formulas
//! called one by one: the same points through the marcher, with the loop
//! (`MB3D_JIT=force` is set) and without it; iteration counts and fields
//! must be equal (NaN payloads aside).
//! `cargo run --release --example pascalfused -- <MB3D dir> SCENES...`
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    mb3d::formulas::add_formula_dir(std::path::PathBuf::from(&a[0]).join("M3Formulas"));
    for f in &a[1..] {
        let (sc, _) = mb3d::animfile::load_scene(std::path::Path::new(f)).unwrap();
        let p = mb3d::calc::CalcParams::new(&sc).unwrap();
        let fused = p.machine.as_ref().is_some_and(|m| m.jit.is_some());
        let mut q = p.clone();
        if let Some(m) = q.machine.as_mut() {
            m.jit = None;
        }
        let (mut m1, mut m2) = (mb3d::calc::Marcher::new(&p, 1), mb3d::calc::Marcher::new(&q, 1));
        let ne = |x: f64, y: f64| x.to_bits() != y.to_bits() && !(x.is_nan() && y.is_nan());
        let mut nd = 0;
        let mut first = String::new();
        for k in 0..20000u64 {
            let s = (k % 2000) as f64 / 2000.0;
            let mut c = [0.0; 3];
            for (i, ci) in c.iter_mut().enumerate() {
                *ci = p.ystart[i] + p.vgrads[0][i] * (p.width as f64 * (0.3 + 0.4 * (k / 2000) as f64 / 10.0)) + p.vgrads[1][i] * (p.height as f64 * 0.5) + p.vgrads[2][i] * s * 400.0;
            }
            let (n1, n2) = (m1.iterations_at(c), m2.iterations_at(c));
            let (x, y) = (&m1.it, &m2.it);
            if n1 != n2 || ne(x.rout, y.rout) || x.v.iter().zip(&y.v).any(|(a, b)| ne(*a, *b)) || ne(x.otrap, y.otrap) || x.j.iter().zip(&y.j).any(|(a, b)| ne(*a, *b)) {
                if nd == 0 {
                    first = format!("point {k}: its {n1} {n2} rout {} {} v {:?} {:?}", x.rout, y.rout, x.v, y.v);
                }
                nd += 1;
            }
        }
        println!("{:40} compiled loop {}: {nd} of 20000 points differ {first}", std::path::Path::new(f).file_name().unwrap().to_string_lossy(), if fused { "used" } else { "NOT used" });
    }
}
