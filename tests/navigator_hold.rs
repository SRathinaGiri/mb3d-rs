//! The Navigator's move buttons held down, with the whole program, without
//! windows on the screen (its own test process: the program's settings
//! would disturb the library's tests).

#![cfg(feature = "gui")]

use mb3d::vcl::input::Raw;
use mb3d::vcl::MouseButton;
use std::time::{Duration, Instant};

const F: &str = "FNavigator";

fn clicks(v: &[Raw]) -> usize {
    v.iter().filter(|r| r.name == "OnClick").count()
}

/// A user's report, 2026-11-18: the move buttons had to be clicked again and
/// again.  Held, a move button clicks when pressed, then again and again
/// until it is let go; its release clicks no more.
#[test]
fn a_held_move_button_goes_on_moving() {
    let (mut ui, mut app) = mb3d::app::build(&[]).unwrap().unwrap();
    ui.show(F);
    app.process(&mut ui);
    let r = ui.ctl_rect(F, "SpeedButton5");
    let (x, y) = (r.x + r.w / 2, r.y + r.h / 2);
    let f = ui.fm(F);
    assert_eq!(clicks(&f.mouse_down(x, y, MouseButton::Left, 0)), 1, "pressed");
    let t = Instant::now();
    let n: usize = (1..=20).map(|i| clicks(&f.tick(t + Duration::from_millis(100 * i)))).sum();
    assert!(n >= 10, "held two seconds: {n} more clicks");
    assert_eq!(clicks(&f.mouse_up(x, y, MouseButton::Left, 0)), 0, "let go");
    assert_eq!(clicks(&f.tick(t + Duration::from_secs(5))), 0, "after letting go");
    // the other buttons click once, on release
    let r = ui.ctl_rect(F, "SpeedButton11");
    let (x, y) = (r.x + r.w / 2, r.y + r.h / 2);
    let f = ui.fm(F);
    assert_eq!(clicks(&f.mouse_down(x, y, MouseButton::Left, 0)), 0);
    assert_eq!(clicks(&f.mouse_up(x, y, MouseButton::Left, 0)), 1);
}
