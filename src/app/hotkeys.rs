//! Hotkeys that bring a window to the front from any window, and one that
//! closes the window it is pressed in (HotkeysGUI.dfm, new in this port):
//! one key combination each, chosen by the user
//! and kept in the ini file (`Hotkey.Navigator  Ctrl+2`, empty: none),
//! saved as soon as it is set.

use super::Mb3d;
use crate::vcl::form::{SS_ALT, SS_CTRL, SS_SHIFT};
use crate::vcl::lang::tr;
use crate::vcl::{Ev, Event, Ui};

const HK: &str = "HotkeysFrm";

/// The windows a hotkey brings up: the ini name, the form (none: closes
/// the window the key is pressed in), the key combination it has unless
/// the user chose another.
const ACTIONS: [(&str, &str, &str); 8] = [
    ("Main", "Mand3DForm", "Ctrl+1"),
    ("Navigator", "FNavigator", "Ctrl+2"),
    ("Animation", "AnimationForm", "Ctrl+3"),
    ("Formulas", "FormulaGUIForm", "Ctrl+4"),
    ("Lighting", "LightAdjustForm", "Ctrl+5"),
    ("PostProcessing", "PostProForm", "Ctrl+6"),
    ("MonteCarlo", "MCForm", "Ctrl+7"),
    ("CloseWindow", "", "Ctrl+W"),
];

/// A key combination: the virtual key code and the Ctrl / Alt / Shift state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Combo {
    pub key: u16,
    pub mods: u8,
}

const MODS: u8 = SS_CTRL | SS_ALT | SS_SHIFT;

/// The name of a key (Windows virtual key codes, as the forms use).
fn key_name(k: u16) -> String {
    match k {
        48..=57 | 65..=90 => (k as u8 as char).to_string(),
        112..=135 => format!("F{}", k - 111),
        96..=105 => format!("Num{}", k - 96),
        8 => "Backspace".into(),
        9 => "Tab".into(),
        13 => "Enter".into(),
        27 => "Esc".into(),
        32 => "Space".into(),
        33 => "PageUp".into(),
        34 => "PageDown".into(),
        35 => "End".into(),
        36 => "Home".into(),
        37 => "Left".into(),
        38 => "Up".into(),
        39 => "Right".into(),
        40 => "Down".into(),
        45 => "Insert".into(),
        46 => "Delete".into(),
        106 => "Num*".into(),
        107 => "Num+".into(),
        109 => "Num-".into(),
        110 => "Num.".into(),
        111 => "Num/".into(),
        _ => format!("Key{k}"),
    }
}

impl Combo {
    pub fn text(&self) -> String {
        let mut s = String::new();
        for (m, n) in [(SS_CTRL, "Ctrl+"), (SS_ALT, "Alt+"), (SS_SHIFT, "Shift+")] {
            if self.mods & m != 0 {
                s.push_str(n);
            }
        }
        s + &key_name(self.key)
    }

    /// `Ctrl+Shift+N`, `F9`, `Alt+Num5` ... (none for an empty or unknown text).
    pub fn parse(s: &str) -> Option<Combo> {
        let mut mods = 0;
        let mut rest = s.trim();
        loop {
            let Some((m, r)) = rest.split_once('+').filter(|(m, _)| !m.is_empty()) else { break };
            match m.trim().to_ascii_lowercase().as_str() {
                "ctrl" | "cmd" => mods |= SS_CTRL,
                "alt" => mods |= SS_ALT,
                "shift" => mods |= SS_SHIFT,
                _ => break,
            }
            rest = r;
        }
        let rest = rest.trim();
        (1..=255u16).find(|&k| key_name(k).eq_ignore_ascii_case(rest)).map(|key| Combo { key, mods })
    }
}

/// The key combinations of the windows, as the ini has them.
fn combos(app: &Mb3d) -> Vec<Option<Combo>> {
    ACTIONS.iter().map(|(name, _, default)| Combo::parse(app.ini.get_extra(&format!("Hotkey.{name}")).unwrap_or(default))).collect()
}

/// Modifier keys alone (Shift, Ctrl, Alt, the Windows keys) are no hotkey.
fn is_modifier(key: u16) -> bool {
    matches!(key, 16 | 17 | 18 | 91 | 92 | 93 | 160..=165)
}

/// A key pressed in a window: brings up the window its combination stands
/// for (true: the key is used up).  The hotkeys window, while open, takes
/// the keys to set them.
pub fn hotkey(app: &mut Mb3d, ui: &mut Ui, form: usize, key: u16, shift: u8) -> bool {
    if is_modifier(key) || ui.forms[form].name == HK {
        return false;
    }
    let c = Combo { key, mods: shift & MODS };
    let Some(i) = combos(app).iter().position(|k| *k == Some(c)) else { return false };
    match ACTIONS[i].1 {
        // as its X button does; the main window stays (it would end the program)
        "" if form != ui.main_form => {
            let name = ui.forms[form].name.clone();
            ui.close(&name);
        }
        "" => {}
        f => ui.show(f),
    }
    true
}

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    match e.handler.as_str() {
        "FormShow" => refresh(app, ui),
        "HotkeyEditKeyDown" => {
            let Ev::KeyDown { key, shift } = e.ev else { return };
            let i = ui.tag(HK, &e.sender) as usize;
            if i >= ACTIONS.len() || is_modifier(key) || key == 9 {
                return;
            }
            // Backspace and Delete alone take the hotkey away
            let v = if (key == 8 || key == 46) && shift & MODS == 0 { String::new() } else { Combo { key, mods: shift & MODS }.text() };
            // a combination belongs to one window only
            if !v.is_empty() {
                for (j, (name, ..)) in ACTIONS.iter().enumerate() {
                    if j != i && combos(app)[j].map(|c| c.text()) == Some(v.clone()) {
                        app.ini.set_extra(&format!("Hotkey.{name}"), "");
                    }
                }
            }
            app.ini.set_extra(&format!("Hotkey.{}", ACTIONS[i].0), &v);
            let _ = app.ini.save();
            refresh(app, ui);
        }
        "DefaultsBtnClick" => {
            for (name, _, default) in ACTIONS {
                app.ini.set_extra(&format!("Hotkey.{name}"), default);
            }
            let _ = app.ini.save();
            refresh(app, ui);
        }
        "SaveAndExitBtnClick" => {
            let _ = app.ini.save();
            ui.hide(HK);
        }
        _ => {}
    }
}

/// The edits show the combinations ("none" where there is none).
fn refresh(app: &Mb3d, ui: &mut Ui) {
    for (i, c) in combos(app).into_iter().enumerate() {
        let t = c.map(|c| c.text()).unwrap_or_else(|| tr("none"));
        ui.set_text(HK, &format!("Edit{}", i + 1), &t);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combos_read_and_written() {
        for s in ["Ctrl+1", "F9", "Ctrl+Alt+Shift+N", "Alt+Num5", "Shift+PageDown", "Ctrl+Key186"] {
            assert_eq!(Combo::parse(s).unwrap().text(), s);
        }
        assert_eq!(Combo::parse("ctrl + n"), Some(Combo { key: 78, mods: SS_CTRL }));
        assert_eq!(Combo::parse("Cmd+2"), Some(Combo { key: 50, mods: SS_CTRL }));
        assert_eq!(Combo::parse(""), None);
        assert_eq!(Combo::parse("Ctrl+"), None);
        assert_eq!(Combo::parse("Hyper+X"), None);
    }
}
