//! Prints the WGSL of a Pascal formula (`jit_wgsl`): `pascalwgsl FILE.m3f [--dbl]`.
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let def = mb3d::m3f::M3f::load(std::path::Path::new(&a[0])).unwrap();
    let prog = def.jit.clone().expect("a Pascal formula");
    match mb3d::jit_wgsl::emit(&prog, &def, &def.defaults(), "fm0", a.iter().any(|x| x == "--dbl"), &Default::default()) {
        Ok((w, words)) => {
            print!("{}", w.code);
            println!("// cells {:?}, constants {words:?}", w.record);
        }
        Err(e) => println!("not translated: {e}"),
    }
}
