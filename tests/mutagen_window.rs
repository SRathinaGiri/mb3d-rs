//! The MutaGen window with the whole program, without windows on the
//! screen (its own test process: the program's settings would disturb the
//! library's tests).

use mb3d::app::mutagen_form::{test_panel, test_state};
use mb3d::vcl::{App, Ev};

const F: &str = "MutaGenFrm";

fn run(app: &mut mb3d::app::Mb3d, ui: &mut mb3d::vcl::Ui, until: impl Fn(&mb3d::app::Mb3d) -> bool) {
    for _ in 0..2400 {
        App::idle(app, ui);
        app.process(ui);
        if until(app) {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    panic!("MutaGen did not get there");
}

/// A user's report, 2026-11-18: a variation double-clicked, sent to the main
/// window and calculated there; the window opened again showed the old
/// centre picture.  Send to Main during the generation the double-click
/// started: the window closes, the generation finishes behind it and the
/// variation sent is the centre when the window opens again.
#[test]
fn a_variation_sent_to_main_is_the_centre_when_the_window_opens_again() {
    // (a Julia scene: its variations have Julia values the main window
    // rounds, so its parameters do not match the variation sent as text)
    let file = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/quaternion_julia.m3s").to_string();
    let (mut ui, mut app) = mb3d::app::build(&[file]).unwrap().unwrap();
    let (ui, app) = (&mut ui, &mut app);
    ui.click("Mand3DForm", "MutaGenBtn");
    run(app, ui, |a| !test_state(a, 0).0);
    // (the variations are random: again until one had values the main
    // window rounds)
    let mut rounded = false;
    for _ in 0..8 {
        // double-click the centre: a new generation around it
        ui.post(F, &test_panel(0), "Panel_1DblClick", Ev::DblClick);
        app.process(ui);
        run(app, ui, |a| test_state(a, 1).2.is_some());
        assert!(test_state(app, 0).0, "the generation should still be under way");
        // its first child to the main window while the others are made
        let sent = test_state(app, 1).2.unwrap();
        ui.popup_menu(F, "MainPopupMenu", &test_panel(1), 5, 5);
        ui.post(F, "SendtoMainItm", "SendtoMainItmClick", Ev::Click);
        app.process(ui);
        assert!(!ui.showing(F), "Send to Main closes the window");
        app.make_scene(ui);
        rounded |= app.scene.to_text() != sent;
        run(app, ui, |a| !test_state(a, 0).0);
        assert_eq!(test_state(app, 0).2.as_deref(), Some(sent.as_str()));
        // back from the main window with them: nothing changes
        let n = test_state(app, 0).1;
        ui.click("Mand3DForm", "MutaGenBtn");
        app.process(ui);
        run(app, ui, |a| !test_state(a, 0).0);
        assert_eq!(test_state(app, 0), (false, n, Some(sent)));
        if rounded {
            break;
        }
    }
    assert!(rounded, "no variation the main window rounds in eight");
}
