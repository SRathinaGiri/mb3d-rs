//! Checks a Gaussian splat export against the program's own picture from a
//! direction that is not one of the export's views:
//!
//!     cargo run --release --example splatcheck -- SCENE OUTDIR AZIMUTH ELEVATION [--formulas DIR] [--maps DIR] [--views N] [--size N] [--sh N] [--sigma F] [--around WHERE]
//!
//! writes OUTDIR/splats.ply, OUTDIR/truth.png (the program's picture from
//! the direction: azimuth degrees to the right around the picture's up,
//! elevation degrees up, around the middle or the camera as the export) and
//! prints the camera of that picture in the file's frame as
//! `CAMERA x y z  fx fy fz  ux uy uz  fov_degrees` (eye, forward, up, the
//! vertical field of view), for a splat renderer to show the file from the
//! same place.

use mb3d::splat;

fn main() -> Result<(), String> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let mut pos = Vec::new();
    let mut sp = splat::SplatParams::default();
    let mut it = a.into_iter();
    while let Some(x) = it.next() {
        let mut val = || it.next().ok_or(format!("{x} needs a value"));
        match x.as_str() {
            "--formulas" => mb3d::formulas::add_formula_dir(val()?.into()),
            "--maps" => mb3d::maps::add_map_dir(val()?.into()),
            "--views" => sp.views = val()?.parse().map_err(|_| "--views N")?,
            "--size" => sp.size = val()?.parse().map_err(|_| "--size N")?,
            "--sh" => sp.degree = val()?.parse().map_err(|_| "--sh N")?,
            "--sigma" => sp.sigma = val()?.parse().map_err(|_| "--sigma F")?,
            "--around" => sp.around = splat::Around::parse(&val()?).ok_or("--around middle|camera|auto")?,
            _ => pos.push(x),
        }
    }
    let [scene, out, az, el] = <[String; 4]>::try_from(pos).map_err(|_| "SCENE OUTDIR AZIMUTH ELEVATION")?;
    let out = std::path::PathBuf::from(out);
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let (sc, _) = mb3d::animfile::load_scene(std::path::Path::new(&scene)).map_err(|e| e.to_string())?;
    let r = splat::export(&sc, &sp, &|_, _| {}, &|| false)?;
    let scale = splat::write_ply(&out.join("splats.ply"), &r, sp.degree)?;
    eprintln!("{} splats around the {}, scale {scale}", r.splats.len(), r.around.name());

    // the direction in the scene's view frame (x right, y down, z forward)
    let (az, el): (f64, f64) = (az.parse().map_err(|_| "AZIMUTH")?, el.parse().map_err(|_| "ELEVATION")?);
    let (az, el) = (az.to_radians(), el.to_radians());
    let dir = [-el.cos() * az.sin(), el.sin(), el.cos() * az.cos()];
    // (around the camera: the scene's own field of view, from the export's
    // eye: MB3D's wide views are not pinhole pictures)
    let base = splat::prepare(&sc, 512);
    let s = if r.around == splat::Around::Camera {
        splat::view_from(&base, dir, r.around, r.origin)?
    } else {
        splat::view_from(&base, dir, r.around, [0.0; 3])?
    };
    let (p, gbuf) = mb3d::render::calculate(&s, &|_, _| {})?;
    let rgb = mb3d::render::paint(&s, &p, &gbuf);
    mb3d::png::write_rgb(&out.join("truth.png").to_string_lossy(), 512, 512, &rgb).map_err(|e| e.to_string())?;

    let cam = mb3d::render::paint_camera(&s, &p);
    let e = splat::eye(&s)?;
    let off = [0, 1, 2].map(|k| s.mid[k] - base.mid[k] + e[k] - r.origin[k]);
    let fr = splat::frame(&r.vgrads);
    let to = |v: [f64; 3]| [0, 1, 2].map(|k| fr[k][0] * v[0] + fr[k][1] * v[1] + fr[k][2] * v[2]);
    let eye = to(off).map(|v| v * scale);
    let fwd = to(cam.pixel_dir(255.5, 255.5));
    let m0 = mb3d::math::normalise_matrix_to(1.0, &s.vgrads);
    let up = to(m0[1].map(|v| -v));
    println!(
        "CAMERA {} {} {}  {} {} {}  {} {} {}  {}",
        eye[0],
        eye[1],
        eye[2],
        fwd[0],
        fwd[1],
        fwd[2],
        up[0],
        up[1],
        up[2],
        (cam.fov as f64).to_degrees()
    );
    Ok(())
}
