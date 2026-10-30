//! A note of a crash in `mb3d_crash.txt` (in the program's data folder,
//! else next to the program): the panic's message and backtrace, or the
//! error the program ended with.  A Windows program has no console, and on
//! any system a crash in a long render, or in a swarm worker or an
//! animation running on its own, would vanish with the window; the file
//! is what a user can send.

use std::io::Write;

/// Installs the panic hook of program `name`: every panic (of any thread)
/// goes to the crash file as well as to stderr.
pub fn install(name: &'static str) {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        crate::log::error("crash", &format!("{info}"));
        note(name, &format!("{info}\n{}", std::backtrace::Backtrace::force_capture()));
        default(info);
    }));
}

/// The file the notes go to: `mb3d_crash.txt` in the data folder, else
/// in the current folder.
pub fn file() -> std::path::PathBuf {
    let p = crate::appdirs::data_folder().join("mb3d_crash.txt");
    if std::fs::OpenOptions::new().create(true).append(true).open(&p).is_ok() { p } else { "mb3d_crash.txt".into() }
}

/// Appends `s` to the crash file, with the program, its version and the time.
pub fn note(name: &str, s: &str) {
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(file()) {
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let _ = writeln!(f, "--- {name} {} (unix time {secs}, {})\n{s}", env!("CARGO_PKG_VERSION"), std::env::consts::OS);
        // what happened before it (the log's newest lines)
        let tail = crate::log::tail(50);
        if !tail.is_empty() {
            let _ = writeln!(f, "the log's last lines (logs/ in the data folder):\n{}", tail.join("\n"));
        }
    }
}
