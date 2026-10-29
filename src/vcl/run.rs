//! The event loop: one window per visible form (winit + softbuffer), with
//! the title bar and borders drawn by the toolkit as the VCL styles do.

use super::canvas::Canvas;
use super::control::Kind;
use super::form::*;
use super::input::{vk, wheel_steps, Raw};
use super::paint::caption_buttons;
use super::ui::{App, Request, Ui, Waker};
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize, PhysicalPosition};
use winit::event::{ElementState, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy, OwnedDisplayHandle};
use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};
use winit::window::{CursorIcon, ResizeDirection, Window, WindowId, WindowLevel};

#[derive(Debug, Clone, Copy)]
enum UserEvent {
    Wake,
}

struct Win {
    window: Rc<Window>,
    surface: softbuffer::Surface<OwnedDisplayHandle, Rc<Window>>,
    form: usize,
    cursor: (f64, f64),
    /// non-client part under the mouse (for the press)
    nc: Nc,
    cursor_icon: CursorIcon,
    /// the title last set (X11 cannot read it back: `Window::title` is empty)
    title: String,
    /// the form's outer size (and the user scale) the window last had or
    /// was asked for: the window is only resized when the program changes
    /// the form, never to undo a size the window manager imposed (under
    /// Wine on a tiling manager that bounced between the two sizes forever
    /// and the windows never got painted)
    synced: (i32, i32, f64),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Nc {
    Client,
    Caption,
    /// the language badge of the title bar
    Lang,
    /// the history icon of the title bar
    History,
    Button(i32),
    Border(ResizeDirection),
}

struct Runner<A: App> {
    ui: Ui,
    app: A,
    wins: HashMap<WindowId, Win>,
    by_form: HashMap<usize, WindowId>,
    ctx: Option<softbuffer::Context<OwnedDisplayHandle>>,
    mods: ModifiersState,
    proxy: EventLoopProxy<UserEvent>,
    last_blink: Instant,
    /// caret blinks so far (the title bar LED toggles on every second one)
    blinks: u64,
    last_idle: Instant,
    caption_click: (Instant, usize),
    /// the form in mouse-look mode
    look: Option<usize>,
    buttons: u8,
    /// wheel delta short of a notch (120), for the next event
    wheel_rest: i32,
}

/// The event loop. On Linux X11 is preferred, also on a Wayland desktop
/// (through XWayland): the windows open at their places of the .dfm files,
/// and Wayland does not let a program position its windows. `MB3D_WAYLAND=1` uses Wayland anyway.
fn event_loop() -> Result<EventLoop<UserEvent>, String> {
    #[cfg(all(unix, not(target_os = "macos")))]
    if std::env::var_os("MB3D_WAYLAND").is_none() && x11_reachable() {
        use winit::platform::x11::EventLoopBuilderExtX11;
        return EventLoop::<UserEvent>::with_user_event().with_x11().build().map_err(|e| format!("cannot open a window: {e}"));
    }
    EventLoop::<UserEvent>::with_user_event().build().map_err(|e| format!("cannot open a window: {e}"))
}

/// Whether the X server of `DISPLAY` (a local ":N") accepts connections
/// (winit creates its event loop only once, there is no second attempt).
#[cfg(all(unix, not(target_os = "macos")))]
fn x11_reachable() -> bool {
    let Ok(d) = std::env::var("DISPLAY") else { return false };
    let Some(n) = d.strip_prefix(':').map(|r| r.split('.').next().unwrap_or("")) else { return false };
    !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) && std::os::unix::net::UnixStream::connect(format!("/tmp/.X11-unix/X{n}")).is_ok()
}

/// Runs the program until the main form closes.
pub fn run<A: App + 'static>(mut ui: Ui, app: A) -> Result<(), String> {
    let el = event_loop()?;
    let proxy = el.create_proxy();
    let p2 = proxy.clone();
    ui.waker = Some(Waker(Arc::new(move || {
        let _ = p2.send_event(UserEvent::Wake);
    })));
    let mut r = Runner {
        ui,
        app,
        wins: HashMap::new(),
        by_form: HashMap::new(),
        ctx: None,
        mods: ModifiersState::empty(),
        proxy,
        last_blink: Instant::now(),
        blinks: 0,
        last_idle: Instant::now(),
        caption_click: (Instant::now() - Duration::from_secs(10), usize::MAX),
        look: None,
        buttons: 0,
        wheel_rest: 0,
    };
    el.run_app(&mut r).map_err(|e| e.to_string())
}

fn shift_of(m: ModifiersState) -> u8 {
    let mut s = 0;
    if m.shift_key() {
        s |= SS_SHIFT;
    }
    if m.control_key() || m.super_key() && cfg!(target_os = "macos") {
        s |= SS_CTRL;
    }
    if m.alt_key() {
        s |= SS_ALT;
    }
    s
}

/// Windows virtual key code of a physical key.
pub fn vk_of(k: KeyCode) -> u16 {
    use KeyCode::*;
    match k {
        KeyA => 0x41,
        KeyB => 0x42,
        KeyC => 0x43,
        KeyD => 0x44,
        KeyE => 0x45,
        KeyF => 0x46,
        KeyG => 0x47,
        KeyH => 0x48,
        KeyI => 0x49,
        KeyJ => 0x4A,
        KeyK => 0x4B,
        KeyL => 0x4C,
        KeyM => 0x4D,
        KeyN => 0x4E,
        KeyO => 0x4F,
        KeyP => 0x50,
        KeyQ => 0x51,
        KeyR => 0x52,
        KeyS => 0x53,
        KeyT => 0x54,
        KeyU => 0x55,
        KeyV => 0x56,
        KeyW => 0x57,
        KeyX => 0x58,
        KeyY => 0x59,
        KeyZ => 0x5A,
        Digit0 => 0x30,
        Digit1 => 0x31,
        Digit2 => 0x32,
        Digit3 => 0x33,
        Digit4 => 0x34,
        Digit5 => 0x35,
        Digit6 => 0x36,
        Digit7 => 0x37,
        Digit8 => 0x38,
        Digit9 => 0x39,
        Numpad0 => 0x60,
        Numpad1 => 0x61,
        Numpad2 => 0x62,
        Numpad3 => 0x63,
        Numpad4 => 0x64,
        Numpad5 => 0x65,
        Numpad6 => 0x66,
        Numpad7 => 0x67,
        Numpad8 => 0x68,
        Numpad9 => 0x69,
        NumpadMultiply => 0x6A,
        NumpadAdd => 0x6B,
        NumpadSubtract => 0x6D,
        NumpadDecimal => 0x6E,
        NumpadDivide => 0x6F,
        NumpadEnter | Enter => vk::RETURN,
        F1 => 0x70,
        F2 => 0x71,
        F3 => 0x72,
        F4 => 0x73,
        F5 => 0x74,
        F6 => 0x75,
        F7 => 0x76,
        F8 => 0x77,
        F9 => 0x78,
        F10 => 0x79,
        F11 => 0x7A,
        F12 => 0x7B,
        Space => vk::SPACE,
        Escape => vk::ESCAPE,
        Backspace => vk::BACK,
        Tab => vk::TAB,
        Delete => vk::DELETE,
        Insert => vk::INSERT,
        Home => vk::HOME,
        End => vk::END,
        PageUp => vk::PRIOR,
        PageDown => vk::NEXT,
        ArrowLeft => vk::LEFT,
        ArrowRight => vk::RIGHT,
        ArrowUp => vk::UP,
        ArrowDown => vk::DOWN,
        ShiftLeft | ShiftRight => vk::SHIFT,
        ControlLeft | ControlRight => vk::CONTROL,
        AltLeft | AltRight => vk::MENU,
        Minus => 0xBD,
        Equal => 0xBB,
        Comma => 0xBC,
        Period => 0xBE,
        Slash => 0xBF,
        Semicolon => 0xBA,
        BracketLeft => 0xDB,
        BracketRight => 0xDD,
        Backslash => 0xDC,
        Quote => 0xDE,
        Backquote => 0xC0,
        _ => 0,
    }
}

fn cursor_for(name: &str) -> CursorIcon {
    match name {
        "crHandPoint" => CursorIcon::Pointer,
        "crCross" => CursorIcon::Crosshair,
        "crIBeam" => CursorIcon::Text,
        "crSizeAll" | "crSize" => CursorIcon::Move,
        "crSizeWE" | "crHSplit" => CursorIcon::EwResize,
        "crSizeNS" | "crVSplit" => CursorIcon::NsResize,
        "crHourGlass" | "crSQLWait" => CursorIcon::Wait,
        "crAppStart" => CursorIcon::Progress,
        "crNo" => CursorIcon::NotAllowed,
        "crDrag" | "crMultiDrag" => CursorIcon::Grabbing,
        "crHelp" => CursorIcon::Help,
        _ => CursorIcon::Default,
    }
}

impl<A: App> Runner<A> {
    /// Logical (.dfm) pixels to device pixels: the screen's scale factor
    /// times the user's interface scale.
    fn scale_of(&self, w: &Win) -> f64 {
        w.window.scale_factor() * self.ui.ui_scale
    }

    /// Hands the queued events to the program until none are left.
    fn process(&mut self) {
        for _ in 0..10000 {
            let Some(e) = self.ui.take_event() else { break };
            if self.ui.internal_result(&e) {
                continue;
            }
            self.app.event(&mut self.ui, &e);
        }
        // a modal form opened: the others lose the mouse capture (their
        // release would never come) and stop auto-repeating
        if let Some(m) = self.ui.modal_top() {
            for (i, f) in self.ui.forms.iter_mut().enumerate() {
                if i != m && (f.capture.is_some() || f.repeat_at.is_some()) {
                    f.capture = None;
                    f.cap_button = None;
                    f.cap_part = PART_NONE;
                    f.cap_value = 0;
                    f.repeat_at = None;
                    f.dirty = true;
                }
            }
        }
    }

    fn dispatch(&mut self, fi: usize, raws: Vec<Raw>) {
        self.ui.dispatch(fi, raws);
        self.process();
    }

    /// Creates / closes windows for the forms' visibility and applies the
    /// size changes made by the program.
    fn sync(&mut self, el: &ActiveEventLoop) {
        if self.ctx.is_none() {
            self.ctx = softbuffer::Context::new(el.owned_display_handle()).ok();
        }
        let theme = self.ui.theme;
        for fi in 0..self.ui.forms.len() {
            let visible = self.ui.forms[fi].visible;
            match (visible, self.by_form.get(&fi).copied()) {
                (true, None) => self.create_window(el, fi),
                (false, Some(wid)) => {
                    self.wins.remove(&wid);
                    self.by_form.remove(&fi);
                }
                (true, Some(wid)) => {
                    let f = &mut self.ui.forms[fi];
                    let w = self.wins.get_mut(&wid).unwrap();
                    if f.dirty {
                        f.layout();
                        w.window.request_redraw();
                    }
                    let (ow, oh) = f.outer_size(&theme);
                    let us = self.ui.ui_scale;
                    let s = w.window.scale_factor() * us;
                    let cur: LogicalSize<f64> = w.window.inner_size().to_logical(s);
                    let changed = w.synced != (ow, oh, us);
                    w.synced = (ow, oh, us);
                    if changed && !f.maximized && ((cur.width - ow as f64).abs() > 1.0 || (cur.height - oh as f64).abs() > 1.0) {
                        if std::env::var_os("MB3D_TRACE_REDRAW").is_some() {
                            trace_line(&format!("sync {}: request {}x{} (now {:.0}x{:.0} logical, scale {})", f.name, ow, oh, cur.width, cur.height, s));
                        }
                        let _ = w.window.request_inner_size(LogicalSize::new(ow as f64 * us, oh as f64 * us));
                    }
                    let title = f.caption();
                    if w.title != title {
                        w.window.set_title(&title);
                        w.title = title.to_string();
                    }
                }
                _ => {}
            }
        }
        for r in std::mem::take(&mut self.ui.requests) {
            let fi = match r {
                Request::Raise(i) | Request::Minimize(i) | Request::Maximize(i, _) | Request::Move(i, ..) | Request::Cursor(i, _) | Request::MouseLook(i, _) => i,
            };
            let Some(w) = self.by_form.get(&fi).and_then(|id| self.wins.get(id)) else { continue };
            match r {
                Request::Raise(_) => {
                    w.window.set_minimized(false);
                    w.window.focus_window();
                }
                Request::Minimize(_) => w.window.set_minimized(true),
                Request::Maximize(_, m) => w.window.set_maximized(m),
                Request::Move(_, x, y) => w.window.set_outer_position(LogicalPosition::new(x, y)),
                Request::Cursor(_, c) => w.window.set_cursor(cursor_for(c)),
                Request::MouseLook(_, on) => {
                    use winit::window::CursorGrabMode;
                    if on {
                        let _ = w.window.set_cursor_grab(CursorGrabMode::Locked).or_else(|_| w.window.set_cursor_grab(CursorGrabMode::Confined));
                        w.window.set_cursor_visible(false);
                        self.look = Some(fi);
                    } else {
                        let _ = w.window.set_cursor_grab(CursorGrabMode::None);
                        w.window.set_cursor_visible(true);
                        self.look = None;
                    }
                }
            }
        }
        // a control's cursor can change without the mouse moving (the
        // image's hourglass while a calculation runs): it would otherwise
        // stay until the next mouse move
        let ids: Vec<WindowId> = self.wins.keys().copied().collect();
        for id in ids {
            self.update_cursor(id);
        }
        if self.ui.quit {
            el.exit();
        }
    }

    fn create_window(&mut self, el: &ActiveEventLoop, fi: usize) {
        let theme = self.ui.theme;
        let us = self.ui.ui_scale;
        // Auxiliary VCL forms are dialogs owned by the main form.  On
        // Windows this keeps the application's taskbar representation to one
        // entry instead of one entry per floating parameter window.
        #[cfg(target_os = "windows")]
        let main_owner = if fi == self.ui.main_form {
            None
        } else {
            use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
            self.by_form
                .get(&self.ui.main_form)
                .and_then(|id| self.wins.get(id))
                .and_then(|w| w.window.window_handle().ok())
                .and_then(|h| match h.as_raw() {
                    RawWindowHandle::Win32(h) => Some(h.hwnd.get()),
                    _ => None,
                })
        };
        let f = &mut self.ui.forms[fi];
        f.layout();
        let (w, h) = f.outer_size(&theme);
        // the window's size on the screen: the user's scale on top of the
        // .dfm size, at most the monitor
        let mon = el.primary_monitor().or_else(|| el.available_monitors().next());
        let (mut sw, mut sh) = (w as f64 * us, h as f64 * us);
        if let Some(m) = &mon {
            let ms: LogicalSize<f64> = m.size().to_logical(m.scale_factor());
            sw = sw.min(ms.width);
            sh = sh.min(ms.height);
        }
        let mut attrs = Window::default_attributes()
            .with_title(f.caption())
            .with_inner_size(LogicalSize::new(sw, sh))
            .with_decorations(false)
            .with_resizable(f.sizeable())
            .with_window_icon(app_icon());
        #[cfg(target_os = "windows")]
        if fi != self.ui.main_form {
            use winit::platform::windows::WindowAttributesExtWindows;
            attrs = attrs.with_skip_taskbar(true);
            if let Some(owner) = main_owner {
                attrs = attrs.with_owner_window(owner);
            }
        }
        if f.sizeable() {
            let b = f.frame_w() * 2;
            let min_w = f.ctl[0].min_w.max(120) + b;
            let min_h = f.ctl[0].min_h.max(60) + b + f.caption_h(&theme);
            attrs = attrs.with_min_inner_size(LogicalSize::new((min_w as f64 * us).min(sw), (min_h as f64 * us).min(sh)));
        }
        if f.stay_on_top {
            attrs = attrs.with_window_level(WindowLevel::AlwaysOnTop);
        }
        // position
        let centre = matches!(f.position.as_str(), "poScreenCenter" | "poDesktopCenter" | "poMainFormCenter" | "poOwnerFormCenter");
        if let Some(m) = &mon {
            let ms: LogicalSize<f64> = m.size().to_logical(m.scale_factor());
            let mp: LogicalPosition<f64> = m.position().to_logical(m.scale_factor());
            let (x, y) = if centre {
                (mp.x + (ms.width - sw) / 2.0, mp.y + (ms.height - sh) / 2.0)
            } else {
                (f.left as f64, f.top as f64)
            };
            let x = x.clamp(mp.x, (mp.x + ms.width - 100.0).max(mp.x));
            let y = y.clamp(mp.y, (mp.y + ms.height - 60.0).max(mp.y));
            attrs = attrs.with_position(LogicalPosition::new(x, y));
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            use winit::platform::wayland::WindowAttributesExtWayland;
            use winit::platform::x11::WindowAttributesExtX11;
            // the app ID (Wayland) and WM_CLASS (X11) name the menu entry,
            // mb3d-rs.desktop: the desktop takes the window's icon from it
            // (Wayland ignores the icon the program sets); in a Flatpak the
            // menu entry is named after the Flatpak's ID
            let app_id = std::env::var("FLATPAK_ID").unwrap_or_else(|_| "mb3d-rs".into());
            attrs = WindowAttributesExtWayland::with_name(attrs, &app_id, "Mandelbulb3D");
            attrs = WindowAttributesExtX11::with_name(attrs, &app_id, "Mandelbulb3D");
        }
        let window = match el.create_window(attrs) {
            Ok(w) => Rc::new(w),
            Err(e) => {
                eprintln!("cannot create a window: {e}");
                self.ui.forms[fi].visible = false;
                return;
            }
        };
        let Some(ctx) = &self.ctx else { return };
        let surface = match softbuffer::Surface::new(ctx, window.clone()) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("cannot draw into the window: {e}");
                return;
            }
        };
        let id = window.id();
        window.request_redraw();
        self.wins.insert(id, Win { window, surface, form: fi, cursor: (0.0, 0.0), nc: Nc::Client, cursor_icon: CursorIcon::Default, title: self.ui.forms[fi].caption().to_string(), synced: (w, h, us) });
        self.by_form.insert(fi, id);
        f_created(&mut self.ui, fi);
    }

    fn redraw(&mut self, id: WindowId) {
        let theme = self.ui.theme;
        let Some(w) = self.wins.get_mut(&id) else { return };
        let size = w.window.inner_size();
        let trace = std::env::var_os("MB3D_TRACE_REDRAW").is_some();
        if trace {
            trace_line(&format!("redraw {} {}x{}", w.title, size.width, size.height));
        }
        let (Some(pw), Some(ph)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else { return };
        let scale = (w.window.scale_factor() * self.ui.ui_scale) as f32;
        let f = &mut self.ui.forms[w.form];
        if let Err(e) = w.surface.resize(pw, ph) {
            if trace {
                trace_line(&format!("  resize: {e}"));
            }
            return;
        }
        let mut buf = match w.surface.buffer_mut() {
            Ok(b) => b,
            Err(e) => {
                if trace {
                    trace_line(&format!("  buffer: {e}"));
                }
                return;
            }
        };
        f.layout();
        let lw = (size.width as f32 / scale).round() as i32;
        let lh = (size.height as f32 / scale).round() as i32;
        let mut cv = Canvas::new(&mut buf, size.width as usize, size.height as usize, scale);
        f.paint(&mut cv, &theme, lw, lh);
        f.dirty = false;
        if let Err(e) = buf.present() {
            if trace {
                trace_line(&format!("  present: {e}"));
            }
        }
    }

    /// Non-client hit test in logical window coordinates.
    fn nc_hit(&self, fi: usize, x: i32, y: i32, ww: i32, wh: i32) -> Nc {
        let f = &self.ui.forms[fi];
        let th = &self.ui.theme;
        if !f.has_caption() {
            return Nc::Client;
        }
        let b = f.frame_w();
        if f.sizeable() && !f.maximized {
            let g = 5;
            let (l, r, t, bo) = (x < g, x >= ww - g, y < g, y >= wh - g);
            let dir = match (l, r, t, bo) {
                (true, _, true, _) => Some(ResizeDirection::NorthWest),
                (_, true, true, _) => Some(ResizeDirection::NorthEast),
                (true, _, _, true) => Some(ResizeDirection::SouthWest),
                (_, true, _, true) => Some(ResizeDirection::SouthEast),
                (true, ..) => Some(ResizeDirection::West),
                (_, true, ..) => Some(ResizeDirection::East),
                (_, _, true, _) => Some(ResizeDirection::North),
                (_, _, _, true) => Some(ResizeDirection::South),
                _ => None,
            };
            if let Some(d) = dir {
                return Nc::Border(d);
            }
        }
        let ch = f.caption_h(th);
        if y < b + ch {
            for (k, r) in caption_buttons(f, th, ww) {
                if r.contains(x, y) {
                    return Nc::Button(k);
                }
            }
            if f.lang_rect.get().contains(x, y) {
                return Nc::Lang;
            }
            if !f.history_hint.is_empty() && f.history_rect.get().inset(-3).contains(x, y) {
                return Nc::History;
            }
            return Nc::Caption;
        }
        Nc::Client
    }

    fn input_blocked(&self, fi: usize) -> bool {
        matches!(self.ui.modal_top(), Some(m) if m != fi)
    }

    fn client_xy(&self, fi: usize, x: f64, y: f64) -> (i32, i32) {
        let f = &self.ui.forms[fi];
        let b = f.frame_w();
        let ch = f.caption_h(&self.ui.theme);
        (x.floor() as i32 - b, y.floor() as i32 - b - ch)
    }

    fn update_cursor(&mut self, id: WindowId) {
        let Some(w) = self.wins.get(&id) else { return };
        let f = &self.ui.forms[w.form];
        let icon = match w.nc {
            Nc::Lang | Nc::History => CursorIcon::Pointer,
            Nc::Border(d) => match d {
                ResizeDirection::East | ResizeDirection::West => CursorIcon::EwResize,
                ResizeDirection::North | ResizeDirection::South => CursorIcon::NsResize,
                ResizeDirection::NorthEast | ResizeDirection::SouthWest => CursorIcon::NeswResize,
                _ => CursorIcon::NwseResize,
            },
            Nc::Client => {
                let k = f.capture.or(f.hot);
                match k {
                    Some(k) if f.popup.is_none() => {
                        let c = &f.ctl[k];
                        if !c.cursor.is_empty() && c.cursor != "crDefault" {
                            cursor_for(&c.cursor)
                        } else if matches!(c.kind, Kind::Edit | Kind::Memo) || (c.kind == Kind::ComboBox && c.style.is_empty() && f.hot_part != PART_BUTTON) {
                            CursorIcon::Text
                        } else if self.ui.busy {
                            CursorIcon::Progress
                        } else {
                            CursorIcon::Default
                        }
                    }
                    _ => CursorIcon::Default,
                }
            }
            _ => CursorIcon::Default,
        };
        let w = self.wins.get_mut(&id).unwrap();
        if icon != w.cursor_icon {
            w.cursor_icon = icon;
            w.window.set_cursor(icon);
        }
    }

    /// The window of form `fi` is at `l` now: tells the program ("WMMove").
    fn moved(&mut self, fi: usize, l: LogicalPosition<f64>) {
        let f = &mut self.ui.forms[fi];
        let (x, y) = (l.x.round() as i32, l.y.round() as i32);
        if (x, y) != (f.left, f.top) {
            f.left = x;
            f.top = y;
            let name = f.name.clone();
            self.ui.post(&name, &name, "WMMove", Ev::Move);
            self.process();
        }
    }

    fn mouse_moved(&mut self, id: WindowId, px: f64, py: f64) {
        let Some(w) = self.wins.get(&id) else { return };
        let s = self.scale_of(w);
        let (x, y) = (px / s, py / s);
        let fi = w.form;
        let size: LogicalSize<f64> = w.window.inner_size().to_logical(s);
        let nc = if self.ui.forms[fi].capture.is_some() { Nc::Client } else { self.nc_hit(fi, x as i32, y as i32, size.width as i32, size.height as i32) };
        {
            let w = self.wins.get_mut(&id).unwrap();
            w.cursor = (x, y);
            w.nc = nc;
        }
        let f = &mut self.ui.forms[fi];
        let nh = match nc {
            Nc::Button(k) => k,
            Nc::Lang => NC_LANG,
            Nc::History => NC_HISTORY,
            _ => -1,
        };
        if nh != f.nc_hot {
            f.nc_hot = nh;
            f.dirty = true;
        }
        if self.input_blocked(fi) {
            return;
        }
        let (cx, cy) = self.client_xy(fi, x, y);
        {
            // the hints of the LED and the language badge, at once
            let f = &mut self.ui.forms[fi];
            let r = f.led_rect.get();
            let (xi, yi) = (x as i32, y as i32);
            let on_led = nc == Nc::Caption && f.caption_led.is_some() && !f.led_hint.is_empty() && xi >= r.x - 3 && xi < r.x + r.w + 3 && yi >= r.y - 4 && yi < r.y + r.h + 4;
            let text = if nc == Nc::Lang && !f.lang_hint.is_empty() {
                Some(&f.lang_hint)
            } else if nc == Nc::History && !f.history_hint.is_empty() {
                Some(&f.history_hint)
            } else if on_led {
                Some(&f.led_hint)
            } else {
                None
            };
            if let Some(t) = text {
                if !f.led_hint_shown || f.hint.as_ref().map(|h| &h.0) != Some(t) {
                    f.hint = Some((t.clone(), cx as i32, 2));
                    f.led_hint_shown = true;
                    f.dirty = true;
                }
            } else if f.led_hint_shown {
                f.hint = None;
                f.led_hint_shown = false;
                f.dirty = true;
            }
        }
        let raws = if nc == Nc::Client || self.ui.forms[fi].capture.is_some() {
            // the held buttons too (TShiftState's ssLeft ...): drags check them
            self.ui.forms[fi].mouse_move(cx, cy, shift_of(self.mods) | self.buttons)
        } else {
            // leaving the client area
            let f = &mut self.ui.forms[fi];
            let mut out = Vec::new();
            if let Some(h) = f.hot.take() {
                out.push(Raw { id: h, name: "OnMouseLeave", ev: Ev::MouseLeave });
                if !f.led_hint_shown {
                    f.hint = None;
                }
                f.dirty = true;
            }
            out
        };
        self.dispatch(fi, raws);
        self.update_cursor(id);
        self.redraw_dirty();
    }

    fn redraw_dirty(&mut self) {
        for w in self.wins.values() {
            if self.ui.forms[w.form].dirty {
                w.window.request_redraw();
            }
        }
    }

    fn mouse_button(&mut self, id: WindowId, pressed: bool, b: winit::event::MouseButton) {
        let bit = match b {
            winit::event::MouseButton::Left => SS_LEFT,
            winit::event::MouseButton::Right => SS_RIGHT,
            winit::event::MouseButton::Middle => SS_MIDDLE,
            _ => 0,
        };
        if pressed {
            self.buttons |= bit;
        } else {
            self.buttons &= !bit;
        }
        let Some(w) = self.wins.get(&id) else { return };
        let fi = w.form;
        let (x, y) = w.cursor;
        let nc = w.nc;
        if self.input_blocked(fi) {
            if pressed {
                if let Some(m) = self.ui.modal_top() {
                    self.ui.requests.push(Request::Raise(m));
                }
            }
            return;
        }
        let button = match b {
            winit::event::MouseButton::Left => MouseButton::Left,
            winit::event::MouseButton::Right => MouseButton::Right,
            winit::event::MouseButton::Middle => MouseButton::Middle,
            _ => return,
        };
        let shift = shift_of(self.mods);
        let (cx, cy) = self.client_xy(fi, x, y);
        if pressed {
            match nc {
                Nc::Border(d) if button == MouseButton::Left => {
                    // the window manager takes the press and often its release too
                    self.buttons &= !SS_LEFT;
                    let _ = self.wins[&id].window.drag_resize_window(d);
                    return;
                }
                Nc::Lang if button == MouseButton::Left => {
                    // the program opens its language window
                    let name = self.ui.forms[fi].name.clone();
                    self.ui.post(&name, &name, "LanguageBadgeClick", Ev::Click);
                    self.process();
                    self.redraw_dirty();
                    return;
                }
                Nc::History if button == MouseButton::Left => {
                    // the program opens its History window
                    let name = self.ui.forms[fi].name.clone();
                    self.ui.post(&name, &name, "HistoryIconClick", Ev::Click);
                    self.process();
                    self.redraw_dirty();
                    return;
                }
                Nc::Caption if button == MouseButton::Left => {
                    self.ui.forms[fi].popup = None;
                    let now = Instant::now();
                    if self.caption_click.1 == fi && now.duration_since(self.caption_click.0) < Duration::from_millis(400) && self.ui.forms[fi].sizeable() {
                        self.toggle_max(fi);
                        self.caption_click = (now - Duration::from_secs(10), usize::MAX);
                    } else {
                        self.caption_click = (now, fi);
                        self.buttons &= !SS_LEFT;
                        let _ = self.wins[&id].window.drag_window();
                    }
                    return;
                }
                Nc::Button(k) if button == MouseButton::Left => {
                    self.ui.forms[fi].nc_pressed = k;
                    self.ui.forms[fi].dirty = true;
                    self.redraw_dirty();
                    return;
                }
                Nc::Client => {
                    let raws = self.ui.forms[fi].mouse_down(cx, cy, button, shift);
                    self.dispatch(fi, raws);
                }
                _ => {}
            }
        } else {
            let pressed_nc = self.ui.forms[fi].nc_pressed;
            if pressed_nc >= 0 {
                self.ui.forms[fi].nc_pressed = -1;
                self.ui.forms[fi].dirty = true;
                if nc == Nc::Button(pressed_nc) {
                    match pressed_nc {
                        0 => {
                            self.ui.forms[fi].modal_result = MR_CANCEL;
                            self.ui.close_index(fi);
                            self.process();
                        }
                        1 => self.toggle_max(fi),
                        _ => {
                            self.ui.forms[fi].minimized = true;
                            self.wins[&id].window.set_minimized(true);
                        }
                    }
                }
            } else {
                let raws = self.ui.forms[fi].mouse_up(cx, cy, button, shift);
                self.dispatch(fi, raws);
            }
        }
        self.update_cursor(id);
        self.redraw_dirty();
    }

    fn toggle_max(&mut self, fi: usize) {
        let Some(w) = self.by_form.get(&fi).and_then(|i| self.wins.get(i)) else { return };
        let f = &mut self.ui.forms[fi];
        f.maximized = !f.maximized;
        w.window.set_maximized(f.maximized);
        f.dirty = true;
    }

    fn key(&mut self, id: WindowId, ev: winit::event::KeyEvent) {
        let Some(w) = self.wins.get(&id) else { return };
        let fi = w.form;
        if self.input_blocked(fi) {
            return;
        }
        let code = match ev.physical_key {
            PhysicalKey::Code(c) => vk_of(c),
            _ => 0,
        };
        let shift = shift_of(self.mods);
        if ev.state == ElementState::Pressed {
            if code != 0 && !ev.repeat && self.app.hotkey(&mut self.ui, fi, code, shift) {
                self.process();
                self.redraw_dirty();
                return;
            }
            let mut raws = Vec::new();
            if code != 0 {
                raws = self.ui.forms[fi].key_down(code, shift);
            }
            self.dispatch(fi, raws);
            if let Some(t) = &ev.text {
                // Ctrl+Alt is AltGr; on macOS Option alone types characters (@, [ ...)
                let (ctrl, alt) = (shift & SS_CTRL != 0, shift & SS_ALT != 0);
                if ctrl == alt || alt && cfg!(target_os = "macos") {
                    let raws = self.ui.forms[fi].text_input(t);
                    self.dispatch(fi, raws);
                }
            }
        } else if code != 0 {
            let f = &self.ui.forms[fi];
            let mut raws = Vec::new();
            if f.key_preview {
                raws.push(Raw { id: 0, name: "OnKeyUp", ev: Ev::KeyUp { key: code, shift } });
            }
            if let Some(fc) = f.focus {
                raws.push(Raw { id: fc, name: "OnKeyUp", ev: Ev::KeyUp { key: code, shift } });
            }
            self.dispatch(fi, raws);
        }
        self.redraw_dirty();
    }

    /// Timers, hints, caret blinking, auto-repeat; returns the next wake-up.
    fn housekeeping(&mut self) -> Instant {
        let now = Instant::now();
        let mut next = now + Duration::from_millis(500);
        if let Some(t) = self.ui.run_timers(now) {
            next = next.min(t);
        }
        self.process();
        let blink = now.duration_since(self.last_blink) >= Duration::from_millis(530);
        if blink {
            self.last_blink = now;
            self.blinks += 1;
        }
        let led_tick = blink && self.blinks % 2 == 0;
        next = next.min(self.last_blink + Duration::from_millis(530));
        for fi in 0..self.ui.forms.len() {
            if !self.ui.forms[fi].visible {
                continue;
            }
            let raws = self.ui.forms[fi].tick(now);
            if !raws.is_empty() {
                self.dispatch(fi, raws);
            }
            let f = &mut self.ui.forms[fi];
            if let Some(t) = f.repeat_at {
                next = next.min(t);
            }
            if led_tick && f.led_blink && f.caption_led.is_some() {
                f.led_on = !f.led_on;
                f.dirty = true;
            }
            // caret
            if blink && f.active {
                if let Some(k) = f.focus {
                    if matches!(f.ctl[k].kind, Kind::Edit | Kind::Memo | Kind::ComboBox) {
                        f.caret_on = !f.caret_on;
                        f.dirty = true;
                    }
                }
            }
            // hints
            if f.active && f.capture.is_none() && f.popup.is_none() {
                if let Some(h) = f.hot {
                    let since = now.duration_since(f.hover_since);
                    if f.hint.is_none() && f.hint_for != Some(h) && since >= Duration::from_millis(700) {
                        let mut k = Some(h);
                        while let Some(i) = k {
                            if !f.ctl[i].hint.is_empty() {
                                break;
                            }
                            k = f.ctl[i].parent;
                        }
                        if let Some(i) = k {
                            if f.show_hint(i) && !f.ctl[i].short_hint().trim().is_empty() {
                                f.hint = Some((f.ctl[i].short_hint().to_string(), f.mouse.0, f.mouse.1 + 20));
                                f.hint_for = Some(h);
                                f.hover_since = now;
                                f.dirty = true;
                            }
                        }
                    } else if f.hint.is_some() && since >= Duration::from_secs(8) {
                        f.hint = None;
                        f.dirty = true;
                    } else if f.hint.is_none() && f.hint_for != Some(h) {
                        next = next.min(f.hover_since + Duration::from_millis(700));
                    }
                }
            }
        }
        next
    }
}

fn f_created(ui: &mut Ui, fi: usize) {
    ui.forms[fi].created = true;
}

fn app_icon() -> Option<winit::window::Icon> {
    let b = super::paint::decode_ico(include_bytes!("../../assets/mb3d-rs.ico"), 64)?;
    let rgba: Vec<u8> = b.px.iter().flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8, (p >> 24) as u8]).collect();
    winit::window::Icon::from_rgba(rgba, b.w as u32, b.h as u32).ok()
}

impl<A: App> ApplicationHandler<UserEvent> for Runner<A> {
    fn device_event(&mut self, el: &ActiveEventLoop, _id: winit::event::DeviceId, event: winit::event::DeviceEvent) {
        let Some(fi) = self.look else { return };
        if let winit::event::DeviceEvent::MouseMotion { delta } = event {
            if !self.ui.forms[fi].active {
                return;
            }
            let name = self.ui.forms[fi].name.clone();
            let shift = shift_of(self.mods) | self.buttons;
            self.ui.post(&name, &name, "@MouseDelta", Ev::MouseDelta { dx: delta.0, dy: delta.1, shift });
            self.process();
            self.sync(el);
            self.redraw_dirty();
        }
    }

    fn resumed(&mut self, el: &ActiveEventLoop) {
        if std::env::var_os("MB3D_TRACE_REDRAW").is_some() {
            trace_line("resumed");
        }
        self.process();
        self.sync(el);
    }

    fn user_event(&mut self, el: &ActiveEventLoop, _e: UserEvent) {
        self.app.idle(&mut self.ui);
        self.last_idle = Instant::now();
        self.process();
        self.sync(el);
        self.redraw_dirty();
    }

    fn window_event(&mut self, el: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if std::env::var_os("MB3D_TRACE_REDRAW").is_some() && !matches!(event, WindowEvent::CursorMoved { .. }) {
            trace_line(&format!("event {event:?}"));
        }
        let Some(fi) = self.wins.get(&id).map(|w| w.form) else { return };
        match event {
            WindowEvent::RedrawRequested => self.redraw(id),
            WindowEvent::CloseRequested => {
                if !self.input_blocked(fi) {
                    self.ui.close_index(fi);
                    self.process();
                }
            }
            WindowEvent::Resized(size) => {
                let theme = self.ui.theme;
                let window = self.wins[&id].window.clone();
                let s = window.scale_factor() * self.ui.ui_scale;
                let l: LogicalSize<f64> = size.to_logical(s);
                // a move and a resize in one step (X11) brings no Moved event
                if let Ok(p) = window.outer_position() {
                    self.moved(fi, p.to_logical(window.scale_factor()));
                }
                let f = &mut self.ui.forms[fi];
                f.maximized = window.is_maximized();
                if std::env::var_os("MB3D_TRACE_REDRAW").is_some() {
                    trace_line(&format!("resized {}: {:.0}x{:.0} logical, maximized {}", f.name, l.width, l.height, f.maximized));
                }
                let before = f.client_size();
                f.set_outer_size(l.width.round() as i32, l.height.round() as i32, &theme);
                let (ow, oh) = f.outer_size(&theme);
                let us = self.ui.ui_scale;
                if let Some(w) = self.wins.get_mut(&id) {
                    w.synced = (ow, oh, us);
                }
                let f = &mut self.ui.forms[fi];
                if f.client_size() != before {
                    f.layout();
                    if let Some(h) = f.ctl[0].events.get("OnResize").cloned() {
                        let name = f.name.clone();
                        self.ui.post(&name, &name, &h, Ev::Resize);
                        self.process();
                    }
                }
                window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                self.ui.forms[fi].dirty = true;
            }
            WindowEvent::Moved(p) => {
                let l: LogicalPosition<f64> = p.to_logical(self.wins[&id].window.scale_factor());
                self.moved(fi, l);
            }
            WindowEvent::Focused(on) => {
                let f = &mut self.ui.forms[fi];
                f.active = on;
                f.dirty = true;
                if !on {
                    f.popup = None;
                    f.hint = None;
                }
                let ev = if on { "OnActivate" } else { "OnDeactivate" };
                let raws = vec![Raw { id: 0, name: ev, ev: if on { Ev::Activate } else { Ev::Deactivate } }];
                self.dispatch(fi, raws);
                if on {
                    if let Some(m) = self.ui.modal_top() {
                        if m != fi {
                            self.ui.requests.push(Request::Raise(m));
                        }
                    }
                }
            }
            WindowEvent::ModifiersChanged(m) => self.mods = m.state(),
            WindowEvent::CursorMoved { position, .. } => self.mouse_moved(id, position.x, position.y),
            WindowEvent::CursorLeft { .. } => {
                let f = &mut self.ui.forms[fi];
                if f.capture.is_none() {
                    let mut raws = Vec::new();
                    if let Some(h) = f.hot.take() {
                        raws.push(Raw { id: h, name: "OnMouseLeave", ev: Ev::MouseLeave });
                    }
                    f.nc_hot = -1;
                    f.hint = None;
                    f.led_hint_shown = false;
                    f.mouse = (-1, -1);
                    f.dirty = true;
                    self.dispatch(fi, raws);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => self.mouse_button(id, state == ElementState::Pressed, button),
            WindowEvent::MouseWheel { delta, .. } => {
                if self.input_blocked(fi) {
                    return;
                }
                let d = match delta {
                    MouseScrollDelta::LineDelta(_, y) => (y * 120.0) as i32,
                    MouseScrollDelta::PixelDelta(PhysicalPosition { y, .. }) => (y * 4.0) as i32,
                };
                if d != 0 {
                    let (x, y) = self.wins[&id].cursor;
                    let (cx, cy) = self.client_xy(fi, x, y);
                    let steps = wheel_steps(&mut self.wheel_rest, d);
                    let raws = self.ui.forms[fi].wheel_by(d, steps, cx, cy, shift_of(self.mods));
                    self.dispatch(fi, raws);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => self.key(id, event),
            WindowEvent::DroppedFile(p) => {
                let name = self.ui.forms[fi].name.clone();
                if let Some(h) = self.ui.forms[fi].ctl[0].events.get("OnDragDrop").cloned() {
                    self.ui.post(&name, &name, &h, Ev::DialogResult(DialogResult::File(Some(p))));
                } else {
                    self.ui.post(&name, "@drop", "", Ev::DialogResult(DialogResult::File(Some(p))));
                }
                self.process();
            }
            _ => {}
        }
        self.sync(el);
        self.redraw_dirty();
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        if self.last_idle.elapsed() >= Duration::from_millis(100) {
            self.app.idle(&mut self.ui);
            self.last_idle = Instant::now();
        }
        let next = self.housekeeping();
        self.sync(el);
        self.redraw_dirty();
        el.set_control_flow(ControlFlow::WaitUntil(next.min(Instant::now() + Duration::from_millis(100))));
        let _ = &self.proxy;
    }
}

/// MB3D_TRACE_REDRAW: a line in mb3d_trace.txt (a Windows program has no
/// console for stderr)
fn trace_line(s: &str) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("mb3d_trace.txt") {
        let _ = writeln!(f, "{s}");
    }
}
