fn main() {
    println!("cargo:rerun-if-changed=assets/mb3d-rs.ico");

    // `cfg(jit)`: the formulas compiled at run time (feature "jit"); on
    // macOS only with MB3D_JIT_MACOS=1 at build time, and used there only
    // with MB3D_JIT=1 at run time (not tested there yet: the compiled code
    // needs writable and executable memory; see tools/jit_check.sh)
    println!("cargo::rustc-check-cfg=cfg(jit)");
    println!("cargo:rerun-if-env-changed=MB3D_JIT_MACOS");
    let macos = std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|o| o == "macos");
    let macos_on = std::env::var("MB3D_JIT_MACOS").is_ok_and(|v| v == "1");
    if std::env::var_os("CARGO_FEATURE_JIT").is_some() && (!macos || macos_on) {
        println!("cargo:rustc-cfg=jit");
    }

    // Windows Explorer reads the application icon from the executable's
    // resource section.  The GUI package has its icon beside the program
    // too, but embedding it here is what makes the .exe itself recognizable.
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        winres::WindowsResource::new()
            .set_icon("assets/mb3d-rs.ico")
            .compile()
            .expect("embed the Mandelbulb3D icon in the Windows executable");
    }
}
