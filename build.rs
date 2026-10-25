fn main() {
    println!("cargo:rerun-if-changed=assets/mb3d-rs.ico");

    // `cfg(jit)`: the formulas compiled at run time (feature "jit"), not on
    // macOS (not tested there yet; its Cranelift dependencies are left out
    // there in Cargo.toml)
    println!("cargo::rustc-check-cfg=cfg(jit)");
    let macos = std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|o| o == "macos");
    if std::env::var_os("CARGO_FEATURE_JIT").is_some() && !macos {
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
