//! The Mandelbulb3D desktop application (MB3D's windows).
#![cfg_attr(windows, windows_subsystem = "windows")]
#![allow(non_snake_case)]

fn main() {
    #[cfg(windows)]
    {
        // a Windows program has no console: a panic or an error would
        // vanish, so it also goes to mb3d_crash.txt next to the program
        let default = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            crash_note(&format!("{info}\n{}", std::backtrace::Backtrace::force_capture()));
            default(info);
        }));
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(e) = mb3d::app::run(&args) {
        eprintln!("Mandelbulb3D: {e}");
        #[cfg(windows)]
        crash_note(&format!("Mandelbulb3D: {e}"));
        std::process::exit(1);
    }
}

#[cfg(windows)]
fn crash_note(s: &str) {
    use std::io::Write;
    let p = mb3d::appdirs::app_folder().join("mb3d_crash.txt");
    let p = if std::fs::OpenOptions::new().create(true).append(true).open(&p).is_ok() { p } else { "mb3d_crash.txt".into() };
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let _ = writeln!(f, "--- {} (unix time {secs})\n{s}", env!("CARGO_PKG_VERSION"));
    }
}
