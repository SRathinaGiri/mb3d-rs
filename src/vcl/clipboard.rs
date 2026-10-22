//! Text clipboard (the system clipboard; a process-local one when it is
//! not available, e.g. in tests).

use std::cell::RefCell;

thread_local! {
    static CB: RefCell<Option<arboard::Clipboard>> = const { RefCell::new(None) };
    static LOCAL: RefCell<String> = const { RefCell::new(String::new()) };
}

fn with<R>(f: impl FnOnce(&mut arboard::Clipboard) -> Option<R>) -> Option<R> {
    CB.with(|cb| {
        let mut cb = cb.borrow_mut();
        if cb.is_none() {
            *cb = arboard::Clipboard::new().ok();
        }
        cb.as_mut().and_then(f)
    })
}

/// The text on the clipboard; None when it holds none (an image, files,
/// nothing): the process-local text only without a system clipboard, not
/// the last text copied here after another program copied something else.
pub fn get() -> Option<String> {
    match with(|c| Some(c.get_text().ok())) {
        Some(t) => t,
        None => LOCAL.with(|l| Some(l.borrow().clone())).filter(|s| !s.is_empty()),
    }
}

pub fn set(s: &str) {
    LOCAL.with(|l| *l.borrow_mut() = s.to_string());
    with(|c| c.set_text(s.to_string()).ok());
}
