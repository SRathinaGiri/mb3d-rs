//! Writes `Languages/English.lng`, the complete list of the program's texts
//! (the forms' and the ones it sets while it runs), the file a translation
//! starts from.  `cargo run --example langfile`

fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = root.join("Languages").join("English.lng");
    std::fs::create_dir_all(out.parent().unwrap()).unwrap();
    mb3d::app::lang_form::write_english(&root.join("src"), &out).unwrap();
    println!("{}", out.display());
}
