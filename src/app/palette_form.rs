//! The colour adjustment window (ColorPick.pas, `TColorForm`): the ten
//! surface colours (specular / diffuse / transparency) or the four interior
//! colours on the palette, moved with the sliders below the colour bar.

use super::util::{argb, rgb_of};
use super::Mb3d;
use crate::vcl::bitmap::Bitmap;
use crate::vcl::lang::tr;
use crate::vcl::{DialogResult, Ev, Event, MouseButton, Ui};

const F: &str = "ColorForm";

#[derive(Default)]
pub struct State {
    pub act: i64,
    pub start_x: i32,
    pub rand_mid: [u32; 2],
    pub copy: Option<(Option<[u8; 3]>, [u8; 3], [u8; 3], u8)>,
    pub draw_x: i32,
    /// the colour painted on the image (`DrawColSIndex`, 0..1 on the colour
    /// bar) and whether it is an interior colour (`DrawInOutside`)
    pub draw_index: f32,
    pub draw_inside: bool,
}

fn outside(ui: &Ui) -> bool {
    ui.item_index(F, "RadioGroup1") != 1
}

fn alpha(l: &crate::lighting::Lighting, i: usize) -> u8 {
    l.palette_alpha(i)
}

fn set_alpha(l: &mut crate::lighting::Lighting, i: usize, a: u8) {
    let mut arr: [u8; 10] = std::array::from_fn(|k| l.palette_alpha(k));
    arr[i] = a;
    l.palette_alpha = Some(arr);
}

fn gray(a: u8) -> u32 {
    argb([a, a, a])
}

fn mix(a: [u8; 3], b: [u8; 3], w1: f32) -> u32 {
    argb([0, 1, 2].map(|k| (a[k] as f32 * w1 + b[k] as f32 * (1.0 - w1)).round().clamp(0.0, 255.0) as u8))
}

/// `RepaintImage`: the colour bar and the slider positions / colours.
pub fn repaint_image(app: &mut Mb3d, ui: &mut Ui, update: bool) {
    let out = outside(ui);
    let l = app.scene.lighting.clone();
    let img = ui.c(F, "Image1").rect();
    let (w, h) = (img.w.max(2) as usize, img.h.max(2) as usize);
    let n = if out { 10 } else { 4 };
    let pos = |i: usize| if out { l.palette[i].position } else { l.interior[i].0 } as f32;
    let spec = |i: usize| if out { l.palette[i].specular } else { l.interior[i].1 };
    let tr = |i: usize| if out { alpha(&l, i) } else { l.interior_spec[i] };
    let sm = w as f32 / 32767.0;
    let sw2 = (ui.c(F, "Shape1").width + 1) / 2;
    let mut b = Bitmap::new(w, h, 0xFF00_0000);
    let yh = h * 2 / 5;
    let (mut act, mut from) = (1usize, 0f32);
    let mut to = pos(1) * sm;
    let set_shape = |ui: &mut Ui, nr: usize, left: i32, col: u32| {
        let name = format!("Shape{nr}");
        let c = ui.cm(F, &name);
        c.left = left;
        c.brush_color = col;
    };
    set_shape(ui, 1, img.x - sw2, argb(spec(0)));
    if out {
        set_shape(ui, 11, img.x - sw2, argb(l.palette[0].diffuse));
    }
    set_shape(ui, 21, img.x - sw2, gray(tr(0)));
    for x in 0..w {
        if x as f32 > to && act < n {
            act += 1;
            from = to;
            to = if act >= n { w as f32 - 1.0 } else { pos(act) * sm };
            if to <= from {
                to = from + 1.0;
            }
            let il = img.x + from as i32 - sw2;
            set_shape(ui, act, il, argb(spec(act - 1)));
            if out {
                set_shape(ui, act + 10, il, argb(l.palette[act - 1].diffuse));
            }
            set_shape(ui, act + 20, il, gray(tr(act - 1)));
        }
        let w1 = 1.0 - (x as f32 - from) / (to - from).max(1.0);
        let n2 = if l.no_col_ipol { act - 1 } else if out { act % 10 } else { act & 3 };
        let c_tr = mix([tr(act - 1); 3], [tr(n2); 3], w1);
        let c_sp = mix(spec(act - 1), spec(n2), w1);
        let c_di = if out { mix(l.palette[act - 1].diffuse, l.palette[n2].diffuse, w1) } else { c_sp };
        if x as i32 == app.palette.draw_x {
            ui.cm(F, "Shape31").brush_color = c_di;
        }
        for y in 0..h {
            b.px[y * w + x] = if y < yh / 2 { c_tr } else if y < yh { c_sp } else { c_di };
        }
    }
    if out {
        let left = img.x + (l.palette[9].position as f32 * sm).round() as i32 - sw2;
        set_shape(ui, 10, left, argb(l.palette[9].specular));
        set_shape(ui, 20, left, argb(l.palette[9].diffuse));
        set_shape(ui, 30, left, gray(alpha(&l, 9)));
    } else {
        let left = img.x + (l.interior[3].0 as f32 * sm).round() as i32 - sw2;
        set_shape(ui, 4, left, argb(l.interior[3].1));
        set_shape(ui, 24, left, gray(l.interior_spec[3]));
    }
    ui.set_picture(F, "Image1", Some(b));
    if update && ui.checked(F, "CheckBox2") {
        super::light_form::trigger_repaint(app, ui);
    }
}

fn radio_group1(app: &mut Mb3d, ui: &mut Ui) {
    let out = outside(ui);
    for c in ["Edit2", "Button2", "SpeedButton2", "Label1", "Label2"] {
        ui.set_visible(F, c, out);
    }
    ui.set_visible(F, "Label4", !out);
    ui.set_caption(F, "Label5", &if out { tr("Transparency") } else { tr("Transparency + Specular") });
    for i in (5..=20).chain(25..=30) {
        ui.set_visible(F, &format!("Shape{i}"), out);
    }
    repaint_image(app, ui, false);
}

fn rand_col(r: &mut crate::mutagen::Rng, mid: u32, sat: f32, p: [f32; 3]) -> [u8; 3] {
    let m = [(mid >> 16) as u8, (mid >> 8) as u8, mid as u8];
    std::array::from_fn(|k| {
        let v = (r.f64() as f32).powf(p[k]) * 255.0;
        ((v - m[k] as f32) * sat + m[k] as f32).clamp(0.0, 255.0).round() as u8
    })
}

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    let h = e.handler.as_str();
    match h {
        "FormCreate" => app.palette.rand_mid = [0xFF80_8080, 0xFF80_8080],
        "FormShow" => {
            app.palette.act = 0;
            app.palette.draw_x = -1;
            super::light_form::put_light_in_header(app, ui);
            radio_group1(app, ui);
        }
        "FormHide" => {
            super::light_form::set_from_header(app, ui);
            ui.set_checked(F, "CheckBox3", false);
            super::main_form::show_paint_brush(app, ui, false);
        }
        "RadioGroup1Click" => radio_group1(app, ui),
        "Image1MouseDown" => {
            if let Ev::MouseDown { button: MouseButton::Left, x, .. } = e.ev {
                ui.cm(F, "Shape31").brush_clear = false;
                app.palette.draw_x = x;
                app.palette.draw_inside = !outside(ui);
                app.palette.draw_index = x as f32 / ui.c(F, "Image1").rect().w.max(1) as f32;
                repaint_image(app, ui, false);
            }
        }
        "Shape1MouseDown" => {
            let t = ui.tag(F, &e.sender);
            let Ev::MouseDown { button, .. } = e.ev else { return };
            let out = outside(ui);
            if button == MouseButton::Left {
                app.palette.act = t;
                app.palette.start_x = ui.f(F).mouse.0;
            } else if t < 21 {
                let l = &app.scene.lighting;
                let c = if t > 10 {
                    l.palette[(t - 11) as usize].diffuse
                } else if out {
                    l.palette[(t - 1) as usize].specular
                } else {
                    l.interior[(t - 1) as usize].1
                };
                ui.pick_color(&format!("palette:col:{t}"), argb(c));
            } else {
                let i = (t - 21) as usize;
                let a = if out { alpha(&app.scene.lighting, i) } else { app.scene.lighting.interior_spec[i] };
                ui.input_query(&format!("palette:alpha:{t}"), &tr("Transparency intensity"), &tr("0: no, 255: Full transparency"), &a.to_string());
            }
        }
        "Shape1MouseMove" => {
            let Ev::MouseMove { shift, .. } = e.ev else { return };
            if shift & crate::vcl::form::SS_LEFT == 0 || app.palette.act <= 0 || app.palette.act > 20 {
                return;
            }
            move_slider(app, ui);
        }
        "Shape21MouseEnter" => {
            let t = ui.tag(F, &e.sender);
            let r = ui.c(F, &e.sender).rect();
            let out = outside(ui);
            let a = if (21..=30).contains(&t) {
                let i = (t - 21) as usize;
                if out { alpha(&app.scene.lighting, i) } else { app.scene.lighting.interior_spec[i.min(3)] }
            } else {
                0
            };
            let c = ui.cm(F, "TrackBarEx1");
            c.tag = t;
            c.left = r.right();
            c.position = a as i64;
            c.visible = true;
        }
        "FormMouseMove" => {
            if ui.visible(F, "TrackBarEx1") {
                let r = ui.ctl_rect(F, "TrackBarEx1");
                let (mx, my) = ui.f(F).mouse;
                if !r.contains(mx, my) {
                    ui.set_visible(F, "TrackBarEx1", false);
                    ui.cm(F, "TrackBarEx1").tag = 0;
                }
            }
        }
        "TrackBarEx1Change" => {
            let t = ui.tag(F, "TrackBarEx1");
            let out = outside(ui);
            let ie = if out { 30 } else { 24 };
            if (21..=ie).contains(&t) {
                let a = (ui.position(F, "TrackBarEx1") & 255) as u8;
                let i = (t - 21) as usize;
                if out {
                    set_alpha(&mut app.scene.lighting, i, a);
                } else {
                    app.scene.lighting.interior_spec[i] = a;
                }
                ui.cm(F, &format!("Shape{t}")).brush_color = gray(a);
            }
        }
        "TrackBarEx1Exit" | "TrackBarEx1MouseUp" => {
            if h == "TrackBarEx1Exit" {
                ui.set_visible(F, "TrackBarEx1", false);
            }
            repaint_image(app, ui, true);
        }
        "Button1Click" => {
            // random diffuse colours around the middle colour
            let s = super::util::ptf(&ui.text(F, "Edit1")) as f32;
            let m = app.palette.rand_mid[0];
            let lg = |v: u32| (((v & 255) as f32 / 255.0).clamp(0.05, 0.95)).log(0.5);
            let p = [lg(m >> 16), lg(m >> 8), lg(m)];
            let out = outside(ui);
            let mut rng = crate::mutagen::Rng::from_time();
            let l = &mut app.scene.lighting;
            if out {
                for i in 0..10 {
                    l.palette[i].diffuse = rand_col(&mut rng, m, s, p);
                }
            } else {
                for i in 0..4 {
                    l.interior[i].1 = rand_col(&mut rng, m, s, p);
                }
            }
            repaint_image(app, ui, true);
        }
        "Button2Click" => {
            // random specular colours
            let s = super::util::ptf(&ui.text(F, "Edit2")).clamp(0.0, 3.0) as f32;
            let mut r = crate::mutagen::Rng::from_time();
            let m = rgb_of(app.palette.rand_mid[1]);
            let l = &mut app.scene.lighting;
            for i in 0..10 {
                let r1 = ((r.f64() as f32) * std::f32::consts::TAU).cos() * 0.5 + 0.5;
                let r2 = (s.max(1.0) - r.f64() as f32 * s).min(1.0).max(0.0).sqrt();
                let ct = (255.0 * r1) as f32;
                let sv1: [f32; 3] = std::array::from_fn(|k| (128.0 - ct) * s + m[k] as f32);
                let d = l.palette[i].diffuse;
                l.palette[i].specular = std::array::from_fn(|k| (sv1[k] * r2 + d[k] as f32 * (1.0 - r2)).clamp(0.0, 255.0) as u8);
            }
            repaint_image(app, ui, true);
        }
        "Button3Click" => super::light_form::trigger_repaint(app, ui),
        "SpeedButton1Click" => {
            let t = ui.tag(F, &e.sender).clamp(0, 1) as usize;
            ui.pick_color(&format!("palette:mid:{t}"), app.palette.rand_mid[t]);
        }
        "SpeedButton1MouseDown" => {
            if let Ev::MouseDown { button: MouseButton::Right, .. } = e.ev {
                let t = ui.tag(F, &e.sender).clamp(0, 1) as usize;
                ui.pick_color(&format!("palette:mid:{t}"), app.palette.rand_mid[t]);
            }
        }
        "CheckBox3Click" => {
            // paint the picked colour on the image (the brush on the main window)
            let on = ui.checked(F, "CheckBox3");
            ui.cm(F, "Shape31").brush_clear = !(on && app.palette.draw_x >= 0);
            if on && app.palette.draw_x >= 0 {
                repaint_image(app, ui, false);
            }
            ui.cm(F, "Image1").cursor = if on { "crCross" } else { "crDefault" }.into();
            super::main_form::show_paint_brush(app, ui, on);
        }
        "FormKeyDown" => {
            let Ev::KeyDown { key, .. } = e.ev else { return };
            let (mx, my) = ui.f(F).mouse;
            let Some(id) = ui.f(F).hit(mx, my) else { return };
            let c = &ui.f(F).ctl[id];
            if c.kind != crate::vcl::control::Kind::Shape {
                return;
            }
            let t = c.tag;
            if !(1..=30).contains(&t) {
                return;
            }
            let m = ((t - 1) % 10) as usize;
            let out = outside(ui);
            let l = &mut app.scene.lighting;
            match key {
                67 => {
                    // c: copy the colour
                    let spec = if out { l.palette[m].specular } else { l.interior[m.min(3)].1 };
                    let a = if out { l.palette_alpha(m) } else { l.interior_spec[m.min(3)] };
                    app.palette.copy = Some((Some(l.palette[m].diffuse), spec, l.palette[m].diffuse, a));
                }
                66 | 86 => {
                    // b: paste one colour, v: paste both
                    let Some((dif, spec, _, a)) = app.palette.copy else { return };
                    if key == 86 {
                        if out {
                            if let Some(d) = dif {
                                l.palette[m].diffuse = d;
                            }
                            l.palette[m].specular = spec;
                        } else {
                            l.interior[m.min(3)].1 = spec;
                        }
                    } else if t > 20 {
                        if out {
                            set_alpha(l, m, a);
                        } else {
                            l.interior_spec[m.min(3)] = a;
                        }
                    } else if t > 10 {
                        l.palette[m].diffuse = dif.unwrap_or(spec);
                    } else if out {
                        l.palette[m].specular = spec;
                    } else {
                        l.interior[m.min(3)].1 = spec;
                    }
                    repaint_image(app, ui, true);
                }
                _ => {}
            }
        }
        _ => {}
    }
}

/// `Shape1MouseMove`: moves a colour on the palette (glued or swapping).
fn move_slider(app: &mut Mb3d, ui: &mut Ui) {
    let out = outside(ui);
    let n = if out { 10 } else { 4 };
    let mx = ui.f(F).mouse.0;
    let img = ui.c(F, "Image1").rect();
    let sw2 = (ui.c(F, "Shape1").width + 1) / 2;
    let t0 = app.palette.act;
    let s = ui.c(F, &format!("Shape{t0}")).rect();
    let pt = if t0 > 10 { (t0 - 11) * 2 } else { (t0 - 1) * 2 } as i32;
    let xnew = (s.x + mx - app.palette.start_x).clamp(img.x - sw2 + pt, img.x + img.w - sw2 + pt - (n as i32 - 1) * 2);
    let xplus = xnew - s.x;
    if xplus == 0 {
        return;
    }
    app.palette.start_x += xplus;
    let mut t = if t0 > 10 { (t0 - 11) as usize } else { (t0 - 1) as usize };
    let smul = 32767.0 / img.w.max(1) as f64;
    let l = &mut app.scene.lighting;
    let posof = |l: &crate::lighting::Lighting, i: usize| if out { l.palette[i].position } else { l.interior[i].0 } as i64;
    let set_pos = |l: &mut crate::lighting::Lighting, i: usize, v: i64| {
        let v = v.clamp(0, 32767) as u16;
        if out {
            l.palette[i].position = v
        } else {
            l.interior[i].0 = v
        }
    };
    let pt = posof(l, t);
    set_pos(l, t, pt + (smul * xplus as f64).round() as i64);
    if ui.checked(F, "CheckBox1") {
        // glued: the other colours follow proportionally
        let pt = pt.clamp(1, 32766) as f64;
        for xx in 1..n {
            if xx == t {
                continue;
            }
            let p = posof(l, xx) as f64;
            let mut xp = if xx < t { (smul * xplus as f64 * p / pt).round() } else { (smul * xplus as f64 * (32767.0 - p) / (32767.0 - pt)).round() };
            let lim = (smul * xplus as f64).abs().round();
            if xp.abs() > lim {
                xp = lim * xp.signum();
            }
            set_pos(l, xx, p as i64 + xp as i64);
        }
    } else {
        // keep the order: swap with the neighbours
        loop {
            let xp = posof(l, t);
            if t > 1 && xp < posof(l, t - 1) {
                if out {
                    l.palette.swap(t - 1, t);
                    if let Some(a) = l.palette_alpha.as_mut() {
                        a.swap(t - 1, t);
                    }
                } else {
                    l.interior.swap(t - 1, t);
                    l.interior_spec.swap(t - 1, t);
                }
                t -= 1;
            } else if t < n - 1 && xp > posof(l, t + 1) {
                if out {
                    l.palette.swap(t + 1, t);
                    if let Some(a) = l.palette_alpha.as_mut() {
                        a.swap(t + 1, t);
                    }
                } else {
                    l.interior.swap(t + 1, t);
                    l.interior_spec.swap(t + 1, t);
                }
                t += 1;
            } else {
                break;
            }
        }
        app.palette.act = if t0 > 10 { t as i64 + 11 } else { t as i64 + 1 };
    }
    repaint_image(app, ui, true);
}

pub fn dialog(app: &mut Mb3d, ui: &mut Ui, what: &str, r: &DialogResult) {
    let out = outside(ui);
    match (what.split_once(':'), r) {
        (Some(("col", t)), DialogResult::Color(Some(c))) => {
            let t: i64 = t.parse().unwrap_or(0);
            let c = rgb_of(*c);
            let l = &mut app.scene.lighting;
            if t > 10 {
                l.palette[(t - 11) as usize].diffuse = c;
            } else if out {
                l.palette[(t - 1) as usize].specular = c;
            } else {
                l.interior[(t - 1) as usize].1 = c;
            }
            repaint_image(app, ui, true);
        }
        (Some(("alpha", t)), DialogResult::Text(Some(s))) => {
            let t: i64 = t.parse().unwrap_or(21);
            let a = super::util::pti(s).clamp(0, 255) as u8;
            let i = (t - 21) as usize;
            if out {
                set_alpha(&mut app.scene.lighting, i, a);
            } else {
                app.scene.lighting.interior_spec[i.min(3)] = a;
            }
            repaint_image(app, ui, true);
        }
        (Some(("mid", t)), DialogResult::Color(Some(c))) => {
            let t: usize = t.parse().unwrap_or(0);
            app.palette.rand_mid[t.min(1)] = *c;
            let b = ui.cm(F, &format!("SpeedButton{}", t + 1));
            let mut g = Bitmap::new(15, 14, 0xFFFF_00FF);
            for y in 1..13 {
                for x in 2..14 {
                    g.px[y * 15 + x] = *c;
                }
            }
            b.glyph = Some(g);
            b.num_glyphs = 1;
        }
        _ => {}
    }
}

/// `ModColOnImage`: paints the picked colour on the image under the brush
/// at `x`, `y` (pixels of the calculated image): the colour index of the
/// object pixels around it whose depth is close to the one in the middle
/// is set to the one giving the picked colour there (the palette moves
/// with the depth by "colour var. on Z").  The caller repaints.
pub fn paint_on_image(app: &Mb3d, x: i32, y: i32, radius: i32, rect: bool) -> bool {
    let l = &app.scene.lighting;
    let (index, inside) = (app.palette.draw_index as f64, app.palette.draw_inside);
    let mod_otrap = l.color_on_otrap && !inside;
    let cycling = l.color_cycling;
    // CalcSCstartAndSCmul
    let (mut c_start, mut c_mul) = if inside {
        let d = (l.interior_start as f64).powi(4);
        let mut m = ((l.interior_end as f64).powi(4) - d) * 0.5 * 0.00031605;
        if m.abs() > 1e-10 {
            m = 1.0 / m;
        }
        (d * 0.000158025 + 32768.0, m)
    } else {
        let mut st = ((l.color_start as f64 + 30.0) / 90.0).powi(2) * 32767.0 - 10900.0;
        let mut m = ((l.color_end as f64 + 30.0) / 90.0).powi(2) * 32767.0 - 10900.0 - st;
        if let Some((a1, a2)) = l.fine_col_adj {
            let d = st + m * (a2 as f64 - 30.0) * 0.0166666666666666;
            st += m * (a1 as f64 - 30.0) * 0.0166666666666666;
            m = d - st;
        }
        (st, if m.abs() > 0.001 { 2.0 / m } else if m < 0.0 { -2000.0 } else { 2000.0 })
    };
    c_mul = 1.0 / c_mul;
    let plus = if c_mul < 0.0 { -1.0 } else { 1.0 };
    let in_out: u16 = if inside {
        c_start -= 32768.0;
        0x8000
    } else {
        0
    };
    let var_col_z = l.var_col_z as f64;
    app.eng.edit_gbuffer(|sc, p, g| {
        let (w, h) = (p.rect[2], p.rect[3]);
        if x < 0 || y < 0 || x >= w || y >= h {
            return;
        }
        let ir = if rect { radius - 1 } else { radius };
        let mut irr = ir * ir;
        if !rect && irr < 10 && irr > 3 {
            irr -= 2;
        }
        let at = |x: i32, y: i32| (y * w + x) as usize;
        let s0 = g[at(x, y)];
        if s0.zpos_fine >> 16 > 32767 {
            return;
        }
        let col_zmul = var_col_z * -0.005 / (p.step_width * sc.width as f64);
        let (zcorr, zc_mul) = (1.0 / p.zcorr, 1.0 / p.zc_mul);
        let zz_dif = sc.z_start - sc.mid[2];
        let zz_of = |iz: f64| (((8388351.5 - iz) * zc_mul + 1.0).powi(2) - 1.0) * zcorr;
        let iz0 = (s0.zpos_fine >> 8) as f64;
        let zz0 = zz_of(iz0);
        let max_zdiff = (2.0 * ir as f64 * (1.0 + zz0 * p.de_stop_factor as f64) / (zz0 - (((8388352.5 - iz0) * zc_mul + 1.0).powi(2) - 1.0) * zcorr).abs()).round();
        let col = |zz: f64, idx: f64| ((idx * 2.0 - col_zmul * (zz * p.step_width + zz_dif)) * c_mul + c_start).clamp(-1e9, 1e9).round();
        for iy in -ir..=ir {
            for ix in -ir..=ir {
                let (px, py) = (x + ix, y + iy);
                if px < 0 || py < 0 || px >= w || py >= h || !(rect || iy * iy + ix * ix < irr) {
                    continue;
                }
                let s = &mut g[at(px, py)];
                if s.zpos_fine >> 16 > 32767 {
                    continue;
                }
                let iz = (s.zpos_fine >> 8) as f64;
                if (iz0 - iz).abs() > max_zdiff {
                    continue;
                }
                let zz = zz_of(iz);
                let mut c = col(zz, index);
                if !(0.0..=32767.0).contains(&c) {
                    // (out of the palette: the next round of a cycling one)
                    if !cycling {
                        continue;
                    }
                    c = col(zz, index + if c < 0.0 { plus } else { -plus });
                    if !(0.0..=32767.0).contains(&c) {
                        continue;
                    }
                }
                if mod_otrap {
                    s.otrap = c as u16;
                    s.si_gradient &= 0x7FFF;
                } else {
                    s.si_gradient = c as u16 | in_out;
                }
            }
        }
    })
}
