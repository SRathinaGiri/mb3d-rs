fn main() {
    println!("cargo:rerun-if-changed=assets/mb3d-rs.ico");

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
