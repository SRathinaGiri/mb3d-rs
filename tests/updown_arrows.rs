//! The arrows of an UpDown, with the program's windows (own test process).

use mb3d::vcl::{Ev, MouseButton};

/// MutaGen's generation arrows (udHorizontal) side by side, the right one
/// counting up, as in the VCL; until 0.3.13 every UpDown showed up/down
/// arrows on top of each other.
#[test]
fn horizontal_arrows_count_up_on_the_right() {
    let (mut ui, _app) = mb3d::app::build(&[]).unwrap().unwrap();
    let f = "MutaGenFrm";
    let r = ui.ctl_rect(f, "GenerationBtn");
    for (x, want) in [(r.x + r.w * 3 / 4, true), (r.x + r.w / 4, false)] {
        let y = r.y + r.h / 2;
        let form = ui.fm(f);
        let got: Vec<bool> = form.mouse_down(x, y, MouseButton::Left, 0).iter().filter_map(|e| if let Ev::UpDown { up, .. } = e.ev { Some(up) } else { None }).collect();
        form.mouse_up(x, y, MouseButton::Left, 0);
        assert!(!got.is_empty() && got.iter().all(|&u| u == want), "click at x {x}: {got:?}");
    }
}
