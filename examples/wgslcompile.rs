//! Compile time of a WGSL file (a dumped scene shader, `MB3D_GPU_DUMP`),
//! for experiments with its size: `-- FILE [ENTRY]`.
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let code = std::fs::read_to_string(&a[0]).unwrap();
    let entry = a.get(1).map(String::as_str).unwrap_or("main");
    match mb3d::gpu::compile_time(&code, entry) {
        Ok(t) => println!("{}: {} lines, {t:.2} s", a[0], code.lines().count()),
        Err(e) => println!("{}: {e}", a[0]),
    }
}
