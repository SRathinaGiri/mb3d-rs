//! How long a move in the Navigator waits for its first picture while the
//! picture at rest is being calculated.
//! `cargo run --release --example navlatency -- FILE.m3p [SECONDS...]`
//! (without a display).  For each of SECONDS (how long the camera rested
//! before the move; 0.3 1 2 4 8 when none are given) it prints the time
//! from the move to the first new picture, with the option and without it.
use mb3d::vcl::form::SS_LEFT;
use mb3d::vcl::{Ev, MouseButton};

const NAV: &str = "FNavigator";

fn main() {
    let mut args = std::env::args().skip(1);
    let file = args.next().expect("a parameter file");
    let mut waits: Vec<f64> = args.filter_map(|a| a.parse().ok()).collect();
    if waits.is_empty() {
        waits = vec![0.3, 1.0, 2.0, 4.0, 8.0];
    }
    let (mut ui, mut app) = mb3d::app::build(&[file]).unwrap().unwrap();
    let turn = |ui: &mut mb3d::vcl::Ui, app: &mut mb3d::app::Mb3d, ms: u64| {
        app.process(ui);
        std::thread::sleep(std::time::Duration::from_millis(ms));
        mb3d::vcl::App::idle(app, ui);
        app.process(ui);
    };
    for _ in 0..5 {
        turn(&mut ui, &mut app, 50);
    }
    app.eng.stop();
    app.make_scene(&mut ui);
    let sc = app.scene.clone();
    println!("{} x {}: shadows {}, ambient shadows {}, DEAO {}, reflections {}, depth of field {}", sc.width, sc.height, sc.shadows.is_some(), sc.ao.is_some(), sc.deao.is_some(), sc.mc.reflections, sc.dof.is_some());
    ui.show(NAV);
    // (the window's opening sets the switch from the ini file)
    turn(&mut ui, &mut app, 50);
    let mut worst = [0.0f64; 2];
    for (k, on) in [true, false].into_iter().enumerate() {
        if ui.checked(NAV, "MainLookCBx") != on {
            ui.click(NAV, "MainLookCBx");
        }
        // the whole calculation once: how long the camera has to rest
        let t = std::time::Instant::now();
        turn(&mut ui, &mut app, 50);
        while app.navi.eng.running() {
            turn(&mut ui, &mut app, 20);
        }
        println!("option {}: the picture is whole after {:.2} s", if on { "on" } else { "off" }, t.elapsed().as_secs_f64());
        let mut x = 100;
        for &w in &waits {
            // a move, the camera rests `w` seconds, the next move: until
            // its first picture
            let mut go = |ui: &mut mb3d::vcl::Ui, app: &mut mb3d::app::Mb3d| {
                ui.post(NAV, "Image1", "Image1MouseDown", Ev::MouseDown { x, y: 100, button: MouseButton::Left, shift: SS_LEFT });
                x += 8;
                ui.post(NAV, "Image1", "Image1MouseMove", Ev::MouseMove { x, y: 100, shift: SS_LEFT });
                ui.post(NAV, "Image1", "Image1MouseUp", Ev::MouseUp { x, y: 100, button: MouseButton::Left, shift: 0 });
                app.process(ui);
            };
            go(&mut ui, &mut app);
            let t = std::time::Instant::now();
            while t.elapsed().as_secs_f64() < w {
                turn(&mut ui, &mut app, 5);
            }
            let before = app.navi.eng.picture().map(|p| p.0);
            let busy = app.navi.eng.running();
            let t = std::time::Instant::now();
            go(&mut ui, &mut app);
            loop {
                turn(&mut ui, &mut app, 1);
                let now = app.navi.eng.picture().map(|p| p.0);
                let new = match (&before, &now) {
                    (Some(a), Some(b)) => !std::sync::Arc::ptr_eq(a, b),
                    (None, Some(_)) => true,
                    _ => false,
                };
                if new {
                    break;
                }
                assert!(t.elapsed().as_secs() < 300, "no picture after the move");
            }
            let d = t.elapsed().as_secs_f64();
            worst[k] = worst[k].max(d);
            println!("  rested {w:>4.1} s ({}): first picture {:.0} ms after the move", if busy { "calculating" } else { "at rest" }, d * 1000.0);
        }
    }
    // the keyboard: walking forward (W) as a held key repeats, then a rest
    if ui.checked(NAV, "MainLookCBx") != true {
        ui.click(NAV, "MainLookCBx");
    }
    let key = |ui: &mut mb3d::vcl::Ui, app: &mut mb3d::app::Mb3d, k: u16| {
        ui.post(NAV, "", "FormKeyDown", Ev::KeyDown { key: k, shift: 0 });
        app.process(ui);
    };
    for round in 0..14 {
        let before = app.navi.eng.picture().map(|p| p.0);
        let busy = app.navi.eng.running();
        let t = std::time::Instant::now();
        let mut first = None;
        let mut sent = 0;
        while sent < 6 || first.is_none() {
            if sent < 6 && t.elapsed().as_millis() as usize >= sent * 40 {
                key(&mut ui, &mut app, 87);
                sent += 1;
            }
            turn(&mut ui, &mut app, 1);
            let now = app.navi.eng.picture().map(|p| p.0);
            if first.is_none() && matches!((&before, &now), (Some(a), Some(b)) if !std::sync::Arc::ptr_eq(a, b)) {
                first = Some(t.elapsed().as_secs_f64());
            }
            assert!(t.elapsed().as_secs() < 300, "no picture after the key");
        }
        let d = first.unwrap();
        worst[0] = worst[0].max(d);
        let t = std::time::Instant::now();
        let mut late = false;
        while app.navi.eng.running() && t.elapsed().as_secs() < 120 {
            turn(&mut ui, &mut app, 10);
            // a long calculation: a move 3 s into it
            if !late && t.elapsed().as_secs_f64() > 3.0 {
                late = true;
                let before = app.navi.eng.picture().map(|p| p.0);
                let m = std::time::Instant::now();
                key(&mut ui, &mut app, 87);
                loop {
                    turn(&mut ui, &mut app, 1);
                    let now = app.navi.eng.picture().map(|p| p.0);
                    if matches!((&before, &now), (Some(a), Some(b)) if !std::sync::Arc::ptr_eq(a, b)) {
                        break;
                    }
                    assert!(m.elapsed().as_secs() < 300, "no picture after the key");
                }
                let d = m.elapsed().as_secs_f64();
                worst[0] = worst[0].max(d);
                println!("    a move 3 s into the calculation: first picture after {:.0} ms", d * 1000.0);
            }
        }
        println!("  W x6, round {round} ({}): first picture {:.0} ms, whole after {:.2} s{}; zoom {:.3}", if busy { "calculating" } else { "at rest" }, d * 1000.0, t.elapsed().as_secs_f64(), if app.navi.eng.running() { " (not yet)" } else { "" }, app.navi.scene.zoom);
        // every other round moves on while the picture at rest is calculated
        if round % 2 == 1 {
            key(&mut ui, &mut app, 87);
            let t = std::time::Instant::now();
            while t.elapsed().as_secs_f64() < 0.6 {
                turn(&mut ui, &mut app, 5);
            }
        }
    }
    app.navi.eng.stop();
    println!("worst: option on {:.0} ms, off {:.0} ms", worst[0] * 1000.0, worst[1] * 1000.0);
}
