//! A minimal sampling profiler (no perf permissions needed): SIGPROF every
//! 100 µs of CPU time records the interrupted instruction address; the
//! addresses are written to OUT for `addr2line`.  Runs the formula
//! benchmark loop of one parameter file on one thread.
//! `cargo run --release --example pcsample -- MB3D_DIR FILE OUT`
//! (Linux x86-64 only.)

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
fn main() {
    eprintln!("pcsample works on Linux x86-64 only");
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod linux {
    use std::sync::atomic::{AtomicUsize, Ordering};

    const N: usize = 1 << 20;
    static mut PCS: [usize; N] = [0; N];
    static COUNT: AtomicUsize = AtomicUsize::new(0);

    #[repr(C)]
    struct SigAction {
        handler: usize,
        flags: u64,
        restorer: usize,
        mask: [u64; 16],
    }
    #[repr(C)]
    struct ItimerVal {
        interval: [i64; 2],
        value: [i64; 2],
    }
    unsafe extern "C" {
        fn sigaction(sig: i32, act: *const SigAction, old: *mut SigAction) -> i32;
        fn setitimer(which: i32, new: *const ItimerVal, old: *mut ItimerVal) -> i32;
    }
    const SIGPROF: i32 = 27;
    const SA_SIGINFO: u64 = 4;
    const SA_RESTART: u64 = 0x1000_0000;

    extern "C" fn on_prof(_sig: i32, _info: *mut u8, ctx: *mut u8) {
        // ucontext_t: uc_flags, uc_link, uc_stack (24 bytes), mcontext gregs; RIP = gregs[16]
        let rip = unsafe { *(ctx.add(40 + 16 * 8) as *const usize) };
        let i = COUNT.fetch_add(1, Ordering::Relaxed);
        if i < N {
            unsafe { PCS[i] = rip };
        }
    }

    pub fn main() {
        let a: Vec<String> = std::env::args().skip(1).collect();
        mb3d::formulas::add_formula_dir(std::path::PathBuf::from(&a[0]).join("M3Formulas"));
        let (mut sc, _) = mb3d::animfile::load_scene(std::path::Path::new(&a[1])).unwrap();
        if let Some(w) = std::env::var("MAXW").ok().and_then(|v| v.parse::<f64>().ok()) {
            if sc.width as f64 > w {
                sc.scale_image(w / sc.width as f64);
            }
        }
        let p = mb3d::calc::CalcParams::new(&sc).unwrap();
        let mut it = p.new_iteration();
        unsafe {
            let act = SigAction {
                handler: on_prof as extern "C" fn(i32, *mut u8, *mut u8) as usize,
                flags: SA_SIGINFO | SA_RESTART,
                restorer: 0,
                mask: [0; 16],
            };
            sigaction(SIGPROF, &act, std::ptr::null_mut());
            let t = ItimerVal {
                interval: [0, 100],
                value: [0, 100],
            };
            setitimer(2, &t, std::ptr::null_mut());
        }
        // ROWS=n: n rows from the middle of the image, marched as the CPU
        // renderer does (all of the calculation, not only the formulas)
        if let Some(rows) = std::env::var("ROWS").ok().and_then(|v| v.parse::<i32>().ok()) {
            let h = p.rect[3];
            for y in (h - rows) / 2..(h + rows) / 2 {
                let mut m = mb3d::calc::Marcher::new(&p, y);
                for x in 0..p.rect[2] {
                    std::hint::black_box(m.march_pixel(x, y));
                }
            }
        }
        for k in 0..(if std::env::var_os("ROWS").is_some() { 0 } else { 2_000_000u64 }) {
            let s = (k % 2000) as f64 / 2000.0;
            for i in 0..3 {
                it.c[i] = p.ystart[i]
                    + p.vgrads[0][i] * (p.width as f64 * 0.5)
                    + p.vgrads[1][i] * (p.height as f64 * 0.5)
                    + p.vgrads[2][i] * s * 400.0;
            }
            it.calc_sit = false;
            it.mand_function(p.mode, &p.slots);
        }
        unsafe {
            let t = ItimerVal {
                interval: [0, 0],
                value: [0, 0],
            };
            setitimer(2, &t, std::ptr::null_mut());
        }
        let n = COUNT.load(Ordering::Relaxed).min(N);
        // addresses relative to the executable's load base (PIE): read /proc/self/maps
        let maps = std::fs::read_to_string("/proc/self/maps").unwrap();
        // (with the absolute addresses: samples outside the executable)
        std::fs::write(format!("{}.maps", a[2]), &maps).unwrap();
        std::fs::write(format!("{}.abs", a[2]), (0..n).map(|i| format!("{:#x}\n", unsafe { PCS[i] })).collect::<String>()).unwrap();
        let exe = std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let base = maps
            .lines()
            .filter(|l| l.ends_with(&exe))
            .filter_map(|l| {
                let mut it = l.split_whitespace();
                let range = it.next()?;
                let _perm = it.next()?;
                let off = usize::from_str_radix(it.next()?, 16).ok()?;
                let start = usize::from_str_radix(range.split('-').next()?, 16).ok()?;
                Some(start - off)
            })
            .min()
            .unwrap();
        let mut out = String::new();
        for i in 0..n {
            let pc = unsafe { PCS[i] };
            out.push_str(&format!("{:#x}\n", pc.wrapping_sub(base)));
        }
        std::fs::write(&a[2], out).unwrap();
        eprintln!("{n} samples");
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn main() {
    linux::main();
}
