//! Intel Open Image Denoise (OIDN, Apache-2.0, https://www.openimagedenoise.org):
//! the neural network noise filter for path traced pictures, here for the
//! Monte Carlo renders.  The library is not built in: it is loaded when
//! first needed, from the folder `oidn` next to the program (or the
//! program's own folder), the folder named by `MB3D_OIDN`, or the
//! system's libraries (e.g. Arch Linux's package `openimagedenoise`).
//! Its own device libraries (the CPU's, and NVIDIA's, AMD's or Intel's
//! cards when they are there) lie next to it; the library picks the
//! fastest device itself.

use std::ffi::{c_char, c_void, CStr, CString};
use std::sync::OnceLock;

type Dev = *mut c_void;
type Filt = *mut c_void;
type Buf = *mut c_void;

/// `OIDN_FORMAT_FLOAT3`
const FLOAT3: i32 = 3;

struct Api {
    _lib: libloading::Library,
    new_device: unsafe extern "C" fn(i32) -> Dev,
    commit_device: unsafe extern "C" fn(Dev),
    release_device: unsafe extern "C" fn(Dev),
    device_error: unsafe extern "C" fn(Dev, *mut *const c_char) -> i32,
    new_buffer: unsafe extern "C" fn(Dev, usize) -> Buf,
    release_buffer: unsafe extern "C" fn(Buf),
    write_buffer: unsafe extern "C" fn(Buf, usize, usize, *const c_void),
    read_buffer: unsafe extern "C" fn(Buf, usize, usize, *mut c_void),
    new_filter: unsafe extern "C" fn(Dev, *const c_char) -> Filt,
    release_filter: unsafe extern "C" fn(Filt),
    set_image: unsafe extern "C" fn(Filt, *const c_char, Buf, i32, usize, usize, usize, usize, usize),
    set_bool: unsafe extern "C" fn(Filt, *const c_char, bool),
    set_int: unsafe extern "C" fn(Filt, *const c_char, i32),
    commit_filter: unsafe extern "C" fn(Filt),
    execute_filter: unsafe extern "C" fn(Filt),
}

// (the function pointers are of a library that stays loaded)
unsafe impl Send for Api {}
unsafe impl Sync for Api {}

/// The library's file names on this system.
fn lib_names() -> &'static [&'static str] {
    if cfg!(windows) {
        &["OpenImageDenoise.dll"]
    } else if cfg!(target_os = "macos") {
        &["libOpenImageDenoise.2.dylib", "libOpenImageDenoise.dylib"]
    } else {
        &["libOpenImageDenoise.so.2", "libOpenImageDenoise.so"]
    }
}

/// The places the library is looked for, in order (a bare name: the
/// system's search).
fn candidates() -> Vec<std::path::PathBuf> {
    let mut dirs = Vec::new();
    if let Some(d) = std::env::var_os("MB3D_OIDN") {
        let d = std::path::PathBuf::from(d);
        dirs.push(d.join("lib"));
        dirs.push(d.join("bin"));
        dirs.push(d);
    }
    if let Some(exe) = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.to_path_buf())) {
        dirs.push(exe.join("oidn").join(if cfg!(windows) { "bin" } else { "lib" }));
        dirs.push(exe.join("oidn"));
        dirs.push(exe.clone());
        // (a macOS app bundle: next to the bundle's Contents/MacOS)
        dirs.push(exe.join("../Frameworks"));
        // (Linux packages: /usr/bin/mb3d next to /usr/lib/mb3d-rs/oidn)
        dirs.push(exe.join("../lib/mb3d-rs/oidn/lib"));
    }
    let mut out = Vec::new();
    for d in dirs {
        for n in lib_names() {
            out.push(d.join(n));
        }
    }
    for n in lib_names() {
        out.push(std::path::PathBuf::from(n));
    }
    out
}

/// Opens the library at `path`. On Windows a DLL given with its folder finds
/// the DLLs it needs (TBB, the core library) in that folder, not only next
/// to the program.
///
/// # Safety
/// As `libloading::Library::new`.
unsafe fn open(path: &std::path::Path) -> Result<libloading::Library, libloading::Error> {
    #[cfg(windows)]
    if path.components().count() > 1 {
        use libloading::os::windows::{Library, LOAD_WITH_ALTERED_SEARCH_PATH};
        // SAFETY: as the caller's
        return unsafe { Library::load_with_flags(path, LOAD_WITH_ALTERED_SEARCH_PATH) }.map(Into::into);
    }
    // SAFETY: as the caller's
    unsafe { libloading::Library::new(path) }
}

fn load() -> Result<Api, String> {
    let mut last = String::new();
    for path in candidates() {
        if path.components().count() > 1 && !path.exists() {
            continue;
        }
        // SAFETY: the library's initialisation has no requirements
        match unsafe { open(&path) } {
            Ok(lib) => return api(lib).map_err(|e| format!("{}: {e}", path.display())),
            Err(e) => last = e.to_string(),
        }
    }
    Err(format!("Intel Open Image Denoise not found ({last})"))
}

fn api(lib: libloading::Library) -> Result<Api, String> {
    macro_rules! f {
        ($n:literal) => {
            // SAFETY: the signatures are those of oidn.h (version 2)
            *unsafe { lib.get($n) }.map_err(|e| e.to_string())?
        };
    }
    Ok(Api {
        new_device: f!(b"oidnNewDevice\0"),
        commit_device: f!(b"oidnCommitDevice\0"),
        release_device: f!(b"oidnReleaseDevice\0"),
        device_error: f!(b"oidnGetDeviceError\0"),
        new_buffer: f!(b"oidnNewBuffer\0"),
        release_buffer: f!(b"oidnReleaseBuffer\0"),
        write_buffer: f!(b"oidnWriteBuffer\0"),
        read_buffer: f!(b"oidnReadBuffer\0"),
        new_filter: f!(b"oidnNewFilter\0"),
        release_filter: f!(b"oidnReleaseFilter\0"),
        set_image: f!(b"oidnSetFilterImage\0"),
        set_bool: f!(b"oidnSetFilterBool\0"),
        set_int: f!(b"oidnSetFilterInt\0"),
        commit_filter: f!(b"oidnCommitFilter\0"),
        execute_filter: f!(b"oidnExecuteFilter\0"),
        _lib: lib,
    })
}

fn the_api() -> Result<&'static Api, String> {
    static API: OnceLock<Result<Api, String>> = OnceLock::new();
    API.get_or_init(load).as_ref().map_err(|e| e.clone())
}

/// Whether the library can be loaded (the menu entries and options that
/// need it are shown with a hint otherwise).
pub fn available() -> bool {
    the_api().is_ok()
}

/// Why the library cannot be used (None: it can).
pub fn unavailable_reason() -> Option<String> {
    the_api().err()
}

/// The images of a filter run: the noisy colour and, optionally, the
/// surfaces' colour without light (albedo) and their normals, each three
/// values per pixel (colour and albedo 0..1, normals -1..1).
pub struct Input<'a> {
    pub width: usize,
    pub height: usize,
    pub color: &'a [f32],
    pub albedo: Option<&'a [f32]>,
    pub normal: Option<&'a [f32]>,
    /// the colour is in sRGB (a painted picture), not linear
    pub srgb: bool,
    /// the colour can be above 1 (high dynamic range)
    pub hdr: bool,
}

/// The filter "RT" (ray tracing) on `inp`: the denoised colour.
pub fn denoise(inp: &Input) -> Result<Vec<f32>, String> {
    let a = the_api()?;
    let (w, h) = (inp.width, inp.height);
    let n = w * h * 3;
    if w == 0 || h == 0 || inp.color.len() != n || inp.albedo.is_some_and(|v| v.len() != n) || inp.normal.is_some_and(|v| v.len() != n) {
        return Err("denoise: image sizes differ".into());
    }
    let bytes = n * 4;
    let c = |s: &str| CString::new(s).unwrap();
    // SAFETY: calls of the C API as documented; every handle made here is
    // released before returning, the buffers hold `bytes` bytes
    unsafe {
        let dev = (a.new_device)(0);
        if dev.is_null() {
            return Err("denoise: no device".into());
        }
        let err = |dev: Dev| -> Option<String> {
            let mut msg: *const c_char = std::ptr::null();
            let code = (a.device_error)(dev, &mut msg);
            (code != 0).then(|| if msg.is_null() { format!("error {code}") } else { CStr::from_ptr(msg).to_string_lossy().into_owned() })
        };
        (a.commit_device)(dev);
        if let Some(e) = err(dev) {
            (a.release_device)(dev);
            return Err(format!("denoise: {e}"));
        }
        let mut bufs = Vec::new();
        let filter = (a.new_filter)(dev, c("RT").as_ptr());
        let mut image = |name: &str, data: &[f32]| {
            let b = (a.new_buffer)(dev, bytes);
            if !b.is_null() {
                (a.write_buffer)(b, 0, bytes, data.as_ptr() as *const c_void);
                (a.set_image)(filter, c(name).as_ptr(), b, FLOAT3, w, h, 0, 0, 0);
                bufs.push(b);
            }
            b
        };
        image("color", inp.color);
        if let Some(al) = inp.albedo {
            image("albedo", al);
            if let Some(nm) = inp.normal {
                image("normal", nm);
            }
        }
        let out = (a.new_buffer)(dev, bytes);
        if !out.is_null() {
            (a.set_image)(filter, c("output").as_ptr(), out, FLOAT3, w, h, 0, 0, 0);
            bufs.push(out);
        }
        (a.set_bool)(filter, c("srgb").as_ptr(), inp.srgb);
        (a.set_bool)(filter, c("hdr").as_ptr(), inp.hdr);
        // the quality for final frames
        (a.set_int)(filter, c("quality").as_ptr(), 6);
        (a.commit_filter)(filter);
        (a.execute_filter)(filter);
        let mut res = vec![0f32; n];
        let e = err(dev);
        if e.is_none() && !out.is_null() {
            (a.read_buffer)(out, 0, bytes, res.as_mut_ptr() as *mut c_void);
        }
        let e = e.or_else(|| err(dev));
        (a.release_filter)(filter);
        for b in bufs {
            (a.release_buffer)(b);
        }
        (a.release_device)(dev);
        match e {
            Some(e) => Err(format!("denoise: {e}")),
            None if out.is_null() => Err("denoise: out of memory".into()),
            None => Ok(res),
        }
    }
}

/// [`denoise`] of a painted picture (RGB, 8 bit per channel).
#[allow(dead_code)]
pub fn denoise_rgb(rgb: &[u8], width: usize, height: usize) -> Result<Vec<u8>, String> {
    let color: Vec<f32> = rgb.iter().map(|&v| v as f32 / 255.0).collect();
    let out = denoise(&Input { width, height, color: &color, albedo: None, normal: None, srgb: true, hdr: false })?;
    Ok(out.iter().map(|&v| (v.clamp(0.0, 1.0) * 255.0).round() as u8).collect())
}

/// The records of a Monte Carlo render with their colours denoised: the
/// filter works on the records' averages of the rays (what more rays
/// converge to), before they are painted (exposure, saturation, soft
/// clipping), so the painted picture keeps its brightness.  Pixels without
/// rays stay as they are.
pub fn denoise_mc(img: &crate::mc::McImage) -> Result<Vec<crate::mc::McRecord>, String> {
    let color: Vec<f32> = img.recs.iter().flat_map(|r| r.col.map(|c| if c.is_finite() { c.max(0.0) } else { 0.0 })).collect();
    let out = denoise(&Input { width: img.width, height: img.height, color: &color, albedo: None, normal: None, srgb: false, hdr: true })?;
    Ok(img
        .recs
        .iter()
        .zip(out.chunks(3))
        .map(|(r, c)| {
            let mut r = *r;
            if r.ray_count > 0 {
                r.col = [c[0], c[1], c[2]];
            }
            r
        })
        .collect())
}

/// The painted picture of a Monte Carlo render, denoised ([`denoise_mc`]).
pub fn denoised_mc_picture(img: &crate::mc::McImage, mc: &crate::scene::McSettings, gamma: f32) -> Result<Vec<u8>, String> {
    Ok(crate::mc::paint_recs(&denoise_mc(img)?, mc, gamma))
}
