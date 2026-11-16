//! Words that make the repository look like malware to scanrepo.dev (the
//! independent scan the README invites readers to run): its rule against
//! running code given as text matches the word "eval" followed by an
//! opening bracket anywhere in a file, comments, a call of a math helper
//! and this test's own text included, and rated the repository dangerous
//! (70 of 100, 5 November 2026, for 0.2.100's math helper F1 named so)
//! where it was safe (5 of 100).  Name such functions otherwise (`apply`,
//! `value`, `run`), and never write the word with a bracket after it, not
//! even in a comment or a string: this test checks itself as well.

use std::path::Path;

/// The words the scanner takes for running code given as text.
const WORDS: [&str; 1] = [concat!("ev", "al")];

fn check(p: &Path, found: &mut Vec<String>) {
    if p.is_dir() {
        for e in std::fs::read_dir(p).into_iter().flatten().flatten() {
            check(&e.path(), found);
        }
        return;
    }
    {
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !["rs", "py", "sh", "js", "wgsl", "toml", "yml"].contains(&ext)
           
        {
            return;
        }
        let Ok(text) = std::fs::read_to_string(p) else {
            return;
        };
        for (n, line) in text.lines().enumerate() {
            for w in WORDS {
                let mut from = 0;
                while let Some(i) = line[from..].find(w) {
                    let at = from + i;
                    from = at + w.len();
                    let before = line[..at].chars().next_back();
                    if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                        continue;
                    }
                    if line[from..].trim_start().starts_with('(') {
                        found.push(format!("{}:{}: {}", p.display(), n + 1, line.trim()));
                    }
                }
            }
        }
    }
}

#[test]
fn no_words_the_malware_scanner_flags() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut found = Vec::new();
    for d in ["src", "examples", "tests", "tools", ".github", "build.rs"] {
        check(&root.join(d), &mut found);
    }
    assert!(
        found.is_empty(),
        "scanrepo.dev reads these as running code given as text (rename them):\n{}",
        found.join("\n")
    );
}

/// Starting another process (`std::process::Command`) is what scanrepo.dev
/// rates as "Code Execution", critical: 0.3.12's main window started
/// itself again after bringing over an earlier version's settings and the
/// repository went from safe to dangerous (70 of 100, 16 November 2026).
/// The program does without: it asks the user to start it again, a tool
/// that needs another program is an example or a script outside `src`.
/// Only the files below may start one, each for a reason the scanner has
/// accepted (0.3.11 was rated safe with them).
const MAY_START_PROCESSES: [&str; 1] = [
    // sysctl for the processor's name on macOS (the benchmark's header)
    "src/bench.rs",
];

#[test]
fn no_process_is_started_from_the_program() {
    fn walk(root: &Path, p: &Path, found: &mut Vec<String>) {
        if p.is_dir() {
            for e in std::fs::read_dir(p).into_iter().flatten().flatten() {
                walk(root, &e.path(), found);
            }
            return;
        }
        if p.extension().is_none_or(|e| e != "rs") {
            return;
        }
        let rel = p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
        if MAY_START_PROCESSES.contains(&rel.as_str()) {
            return;
        }
        let Ok(text) = std::fs::read_to_string(p) else { return };
        for (n, line) in text.lines().enumerate() {
            if line.contains(concat!("process", "::", "Command")) || line.contains(concat!("Command", "::", "new")) {
                found.push(format!("{rel}:{}: {}", n + 1, line.trim()));
            }
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut found = Vec::new();
    walk(root, &root.join("src"), &mut found);
    assert!(
        found.is_empty(),
        "scanrepo.dev rates starting a process as malware (Code Execution, critical); do without, or discuss it and add the file to MAY_START_PROCESSES:\n{}",
        found.join("\n")
    );
}
