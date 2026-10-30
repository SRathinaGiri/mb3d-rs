//! The Mandelbulb3D desktop application (MB3D's windows).
#![cfg_attr(windows, windows_subsystem = "windows")]
#![allow(non_snake_case)]

fn main() {
    #[cfg(feature = "gpu")]
    mb3d::gpu::use_own_driver_cache();
    // a panic or an error goes to mb3d_crash.txt as well (see crash.rs)
    mb3d::crash::install("Mandelbulb3D");
    // the log of the session (logs/ in the data folder; the Log window)
    mb3d::log::init("gui", true);
    mb3d::log::session_start("Mandelbulb3D");
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(e) = mb3d::app::run(&args) {
        eprintln!("Mandelbulb3D: {e}");
        mb3d::log::error("session", &e);
        mb3d::log::end("the session ended with an error");
        mb3d::crash::note("Mandelbulb3D", &e);
        std::process::exit(1);
    }
    mb3d::log::end("the session ended");
}

