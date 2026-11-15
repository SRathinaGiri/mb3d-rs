//! The Navigator's picture when the camera rests against the main window's
//! picture of the same parameters, and the drag that looks and slides.
//! `cargo run --release --example navrestcheck -- FILE.m3p [OUT_FOLDER]`
//! (without a display).  Prints how far the Navigator's picture is from
//! the main picture reduced to its size, with the option and without it;
//! OUT_FOLDER gets the three pictures (main.png, rest.png, fast.png).
use mb3d::vcl::form::{SS_LEFT, SS_RIGHT};
use mb3d::vcl::{Ev, MouseButton};

const NAV: &str = "FNavigator";

/// The mean difference per channel (0..255).
fn diff(a: &[u8], b: &[u8]) -> f64 {
    a.iter().zip(b).map(|(x, y)| (*x as f64 - *y as f64).abs()).sum::<f64>() / a.len().max(1) as f64
}

/// `rgb` (`w` x `h`) at `tw` x `th`: each pixel the mean of the part of
/// the picture it covers (at least one pixel).
fn resized(rgb: &[u8], w: usize, h: usize, tw: usize, th: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(tw * th * 3);
    for y in 0..th {
        let (y0, y1) = (y * h / th, ((y + 1) * h / th).max(y * h / th + 1).min(h));
        for x in 0..tw {
            let (x0, x1) = (x * w / tw, ((x + 1) * w / tw).max(x * w / tw + 1).min(w));
            for c in 0..3 {
                let mut sum = 0usize;
                for yy in y0..y1 {
                    for xx in x0..x1 {
                        sum += rgb[(yy * w + xx) * 3 + c] as usize;
                    }
                }
                out.push((sum / ((y1 - y0) * (x1 - x0))) as u8);
            }
        }
    }
    out
}

fn main() {
    let file = std::env::args().nth(1).expect("a parameter file");
    let out = std::env::args().nth(2);
    let (mut ui, mut app) = mb3d::app::build(&[file]).unwrap().unwrap();
    let turn = |ui: &mut mb3d::vcl::Ui, app: &mut mb3d::app::Mb3d, n: usize| {
        for _ in 0..n {
            app.process(ui);
            std::thread::sleep(std::time::Duration::from_millis(50));
            mb3d::vcl::App::idle(app, ui);
            app.process(ui);
        }
    };
    // until the Navigator's calculation is over: the seconds it took
    let rest = |ui: &mut mb3d::vcl::Ui, app: &mut mb3d::app::Mb3d| {
        let t = std::time::Instant::now();
        turn(ui, app, 2);
        while app.navi.eng.running() {
            turn(ui, app, 1);
            assert!(t.elapsed().as_secs() < 600, "the Navigator does not come to rest");
        }
        turn(ui, app, 2);
        t.elapsed().as_secs_f64()
    };
    turn(&mut ui, &mut app, 5);
    app.eng.stop();
    app.make_scene(&mut ui);
    let sc = app.scene.clone();
    println!(
        "{} x {}: shadows {}, ambient shadows {}, DEAO {}, reflections {}, depth of field {}, DE stop {}",
        sc.width,
        sc.height,
        sc.shadows.is_some(),
        sc.ao.is_some(),
        sc.deao.is_some(),
        sc.mc.reflections,
        sc.dof.is_some(),
        sc.de_stop
    );

    ui.show(NAV);
    rest(&mut ui, &mut app);
    assert!(!ui.checked(NAV, "MainLookCBx"), "the main window's look is off at first");
    ui.click(NAV, "MainLookCBx");
    let t_rest = rest(&mut ui, &mut app);
    assert!(ui.checked(NAV, "MainLookCBx"));
    let (a, w, h) = app.navi.eng.picture().expect("the Navigator's picture");
    // the main window's picture, reduced to the Navigator's size (when the
    // Navigator has the main picture's proportions)
    let m = mb3d::frames::render_scaled(&sc, 1, false, false, &|_, _| {}, &|| false).unwrap();
    // (the Navigator's sizes are multiples of 8: nearly the proportions)
    let near = (m.width as f64 / m.height as f64 / (w as f64 / h as f64) - 1.0).abs() < 0.03;
    let (r, rw, rh) = if near { (resized(&m.rgb, m.width, m.height, w, h), w, h) } else { mb3d::history::downscale(&m.rgb, m.width, m.height, w, h) };
    let same_shape = (rw, rh) == (w, h);

    ui.click(NAV, "MainLookCBx");
    let t_fast = rest(&mut ui, &mut app);
    assert!(!ui.checked(NAV, "MainLookCBx"));
    let (b, bw, bh) = app.navi.eng.picture().unwrap();
    assert_eq!((bw, bh), (w, h));
    println!("Navigator {w} x {h}: at rest after {t_rest:.1} s, without the option {t_fast:.1} s");
    if same_shape {
        let (da, db) = (diff(&a, &r), diff(&b, &r));
        println!("difference to the main picture: at rest {da:.2}, without the option {db:.2}");
        assert!(da <= db, "the picture at rest is no further from the main picture");
    } else {
        println!("the Navigator's view has other proportions ({rw} x {rh} reduced): not compared");
    }
    if let Some(o) = out {
        std::fs::create_dir_all(&o).unwrap();
        mb3d::png::write_rgb(&format!("{o}/main.png"), rw, rh, &r).unwrap();
        mb3d::png::write_rgb(&format!("{o}/rest.png"), w, h, &a).unwrap();
        mb3d::png::write_rgb(&format!("{o}/fast.png"), w, h, &b).unwrap();
    }

    // a drag with the left button looks, with the right one slides; its
    // end is no click (the mouse look stays off)
    ui.click(NAV, "MainLookCBx");
    rest(&mut ui, &mut app);
    let (v0, m0) = (app.navi.scene.vgrads, app.navi.scene.mid);
    let drag = |ui: &mut mb3d::vcl::Ui, app: &mut mb3d::app::Mb3d, button: MouseButton, held: u8| {
        ui.post(NAV, "Image1", "Image1MouseDown", Ev::MouseDown { x: 100, y: 100, button, shift: held });
        for x in [110, 130, 150] {
            ui.post(NAV, "Image1", "Image1MouseMove", Ev::MouseMove { x, y: 100, shift: held });
            turn(ui, app, 1);
        }
        ui.post(NAV, "Image1", "Image1MouseUp", Ev::MouseUp { x: 150, y: 100, button, shift: 0 });
        ui.post(NAV, "Image1", "Image1Click", Ev::Click);
        turn(ui, app, 1);
    };
    let t = std::time::Instant::now();
    drag(&mut ui, &mut app, MouseButton::Left, SS_LEFT);
    assert!(app.navi.scene.vgrads != v0 && app.navi.scene.mid == m0, "the left drag turns the view");
    assert!(!app.navi.look, "the end of a drag does not start the mouse look");
    let v1 = app.navi.scene.vgrads;
    drag(&mut ui, &mut app, MouseButton::Right, SS_RIGHT);
    assert!(app.navi.scene.mid != m0 && app.navi.scene.vgrads == v1, "the right drag slides the camera");
    // (the drags ran while the picture at rest was being calculated: each
    // move stops it, so they take no longer than without it)
    println!("two drags: {:.1} s", t.elapsed().as_secs_f64());
    rest(&mut ui, &mut app);
    let (_, w2, h2) = app.navi.eng.picture().unwrap();
    assert_eq!((w2, h2), (w, h), "the picture at rest has the Navigator's size");
    app.navi.eng.stop();
    println!("ok");
}
