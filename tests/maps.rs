//! The maps the repository ships besides MB3D's own (`M3Maps-community`,
//! `M3Maps-barnes`, see their CREDITS.md): every file is a picture the
//! program reads, its name says what it is (the original MB3D picks the
//! decoder by the name; this program by the content, which hid JPEG and
//! BMP files named `.png`), and none is larger than 4096 pixels a side
//! (a map is kept as three floats a pixel and goes into one buffer on the
//! graphics card: 8192 x 8192 would be 800 MB, more than many cards
//! take).

use std::path::PathBuf;

const MAX_SIDE: usize = 4096;

fn check_folder(name: &str) -> usize {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(name);
    let mut n = 0;
    for e in std::fs::read_dir(&dir).unwrap().flatten() {
        let p = e.path();
        let ext = p.extension().map(|x| x.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
        if !["jpg", "png"].contains(&ext.as_str()) {
            assert!(ext == "md", "{}: maps are .jpg or .png", p.display());
            continue;
        }
        let b = std::fs::read(&p).unwrap();
        let is_png = b.starts_with(&[0x89, b'P', b'N', b'G']);
        let is_jpg = b.starts_with(&[0xFF, 0xD8]);
        assert!((ext == "png" && is_png) || (ext == "jpg" && is_jpg), "{}: the name does not match the content", p.display());
        let img = mb3d::image::decode(&b).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
        assert!(img.width.max(img.height) <= MAX_SIDE, "{}: {}x{} is larger than {MAX_SIDE} a side", p.display(), img.width, img.height);
        n += 1;
    }
    n
}

#[test]
fn the_community_maps_load() {
    assert_eq!(check_folder("M3Maps-community"), 154);
}

/// Geoffrey N. Barnes's maps (numbers 10001 to 13015).
#[test]
fn the_barnes_maps_load() {
    assert_eq!(check_folder("M3Maps-barnes"), 61);
}
