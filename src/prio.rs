//! The priority of the calculation threads (MB3D's "Thread Priority" on the
//! Internal page: Idle, Lowest, Lower, Normal), so that the windows stay
//! responsive while all cores calculate.  The program sets it; the library
//! leaves the threads at normal priority unless told otherwise.
//!
//! Linux: the thread's nice value (new threads inherit it from the thread
//! that starts them).  Windows: `SetThreadPriority` (not inherited, so the
//! worker loops call [`calc_thread`] themselves).  macOS: unchanged.

use std::sync::atomic::{AtomicI32, Ordering};

/// 0 Idle, 1 Lowest, 2 Lower, 3 Normal (the item index of MB3D's combo box).
static LEVEL: AtomicI32 = AtomicI32::new(3);

/// The priority of the calculation threads started from now on.
pub fn set_level(level: i32) {
    LEVEL.store(level.clamp(0, 3), Ordering::Relaxed);
}

/// Gives the calling thread the calculation priority (at the start of a
/// calculation thread).
pub fn calc_thread() {
    let level = LEVEL.load(Ordering::Relaxed);
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        unsafe extern "C" {
            fn setpriority(which: i32, who: u32, prio: i32) -> i32;
        }
        // PRIO_PROCESS with 0: the calling thread (Linux nice values are
        // per thread)
        let nice = [19, 15, 8, 0][level as usize];
        unsafe {
            setpriority(0, 0, nice);
        }
    }
    #[cfg(windows)]
    {
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetCurrentThread() -> *mut std::ffi::c_void;
            fn SetThreadPriority(thread: *mut std::ffi::c_void, priority: i32) -> i32;
        }
        // THREAD_PRIORITY_IDLE, _LOWEST, _BELOW_NORMAL, _NORMAL
        let p = [-15, -2, -1, 0][level as usize];
        unsafe {
            SetThreadPriority(GetCurrentThread(), p);
        }
    }
    #[cfg(not(any(target_os = "linux", target_os = "android", windows)))]
    let _ = level;
}
