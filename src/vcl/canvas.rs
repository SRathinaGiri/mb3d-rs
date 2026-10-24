//! Software canvas: drawing primitives in logical (96 dpi) coordinates on a
//! 0RGB pixel buffer of device pixels.

use super::bitmap::Bitmap;
use super::font::{self, Font};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }
    pub fn right(&self) -> i32 {
        self.x + self.w
    }
    pub fn bottom(&self) -> i32 {
        self.y + self.h
    }
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.right() && y < self.bottom()
    }
    pub fn inset(&self, d: i32) -> Rect {
        Rect::new(self.x + d, self.y + d, self.w - 2 * d, self.h - 2 * d)
    }
    pub fn offset(&self, dx: i32, dy: i32) -> Rect {
        Rect::new(self.x + dx, self.y + dy, self.w, self.h)
    }
    pub fn intersect(&self, o: &Rect) -> Rect {
        let x0 = self.x.max(o.x);
        let y0 = self.y.max(o.y);
        let x1 = self.right().min(o.right());
        let y1 = self.bottom().min(o.bottom());
        Rect::new(x0, y0, (x1 - x0).max(0), (y1 - y0).max(0))
    }
    pub fn is_empty(&self) -> bool {
        self.w <= 0 || self.h <= 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HAlign {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VAlign {
    Top,
    Center,
    Bottom,
}

pub fn blend(dst: u32, src: u32, a: u32) -> u32 {
    if a >= 255 {
        return src & 0xFF_FFFF;
    }
    if a == 0 {
        return dst;
    }
    let ia = 255 - a;
    let r = ((src >> 16 & 255) * a + (dst >> 16 & 255) * ia) / 255;
    let g = ((src >> 8 & 255) * a + (dst >> 8 & 255) * ia) / 255;
    let b = ((src & 255) * a + (dst & 255) * ia) / 255;
    r << 16 | g << 8 | b
}

/// Mixes two colours, `t` = 0 gives `a`.
pub fn mix(a: u32, b: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let ch = |s: u32| {
        let x = (a >> s & 255) as f32;
        let y = (b >> s & 255) as f32;
        ((x + (y - x) * t).round() as u32) << s
    };
    0xFF00_0000 | ch(16) | ch(8) | ch(0)
}

/// Text with the `&` accelerator prefixes removed (`&&` is a literal `&`).
pub fn strip_amp(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '&' {
            if it.peek() == Some(&'&') {
                o.push('&');
                it.next();
            }
            continue;
        }
        o.push(c);
    }
    o
}

/// Logical line height of a font (VCL `TextHeight`).
/// How far a caption may stand over its control before it is made to fit
/// (see [`Canvas::fit`]).
pub const SLACK: i32 = 2;

pub fn line_height(f: &Font) -> i32 {
    f.line_height()
}

pub struct Canvas<'a> {
    pub buf: &'a mut [u32],
    pub bw: i32,
    pub bh: i32,
    pub scale: f32,
    ox: i32,
    oy: i32,
    /// device clip rectangle x0, y0, x1, y1
    clip: [i32; 4],
}

impl<'a> Canvas<'a> {
    pub fn new(buf: &'a mut [u32], bw: usize, bh: usize, scale: f32) -> Canvas<'a> {
        Canvas { buf, bw: bw as i32, bh: bh as i32, scale, ox: 0, oy: 0, clip: [0, 0, bw as i32, bh as i32] }
    }

    pub fn dx(&self, x: i32) -> i32 {
        ((self.ox + x) as f32 * self.scale).round() as i32
    }
    pub fn dy(&self, y: i32) -> i32 {
        ((self.oy + y) as f32 * self.scale).round() as i32
    }
    /// Thickness of a 1 pixel line in device pixels.
    pub fn px1(&self) -> i32 {
        (self.scale.round() as i32).max(1)
    }

    pub fn origin(&self) -> (i32, i32) {
        (self.ox, self.oy)
    }
    pub fn set_origin(&mut self, o: (i32, i32)) {
        self.ox = o.0;
        self.oy = o.1;
    }
    pub fn translate(&mut self, dx: i32, dy: i32) {
        self.ox += dx;
        self.oy += dy;
    }

    pub fn clip(&self) -> [i32; 4] {
        self.clip
    }
    pub fn set_clip(&mut self, c: [i32; 4]) {
        self.clip = c;
    }
    /// Restricts the clip region to `r` (logical); returns the old one.
    pub fn clip_to(&mut self, r: Rect) -> [i32; 4] {
        let old = self.clip;
        let (x0, y0, x1, y1) = (self.dx(r.x), self.dy(r.y), self.dx(r.right()), self.dy(r.bottom()));
        self.clip = [old[0].max(x0), old[1].max(y0), old[2].min(x1), old[3].min(y1)];
        old
    }
    pub fn clip_empty(&self) -> bool {
        self.clip[0] >= self.clip[2] || self.clip[1] >= self.clip[3]
    }

    /// Fills device pixels x0..x1, y0..y1 (clipped).
    pub fn fill_dev(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: u32) {
        let x0 = x0.max(self.clip[0]);
        let y0 = y0.max(self.clip[1]);
        let x1 = x1.min(self.clip[2]);
        let y1 = y1.min(self.clip[3]);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        let a = c >> 24;
        for y in y0..y1 {
            let row = &mut self.buf[(y * self.bw) as usize..][x0 as usize..x1 as usize];
            if a >= 255 {
                row.fill(c & 0xFF_FFFF);
            } else {
                for p in row.iter_mut() {
                    *p = blend(*p, c, a);
                }
            }
        }
    }

    pub fn put_dev(&mut self, x: i32, y: i32, c: u32, a: u32) {
        if x < self.clip[0] || y < self.clip[1] || x >= self.clip[2] || y >= self.clip[3] {
            return;
        }
        let p = &mut self.buf[(y * self.bw + x) as usize];
        *p = blend(*p, c, a * (c >> 24) / 255);
    }

    pub fn fill(&mut self, r: Rect, c: u32) {
        let (x0, y0, x1, y1) = (self.dx(r.x), self.dy(r.y), self.dx(r.right()), self.dy(r.bottom()));
        self.fill_dev(x0, y0, x1, y1, c);
    }

    pub fn hline(&mut self, x0: i32, x1: i32, y: i32, c: u32) {
        let t = self.px1();
        let (a, b, yy) = (self.dx(x0), self.dx(x1), self.dy(y));
        self.fill_dev(a, yy, b, yy + t, c);
    }

    pub fn vline(&mut self, x: i32, y0: i32, y1: i32, c: u32) {
        let t = self.px1();
        let (xx, a, b) = (self.dx(x), self.dy(y0), self.dy(y1));
        self.fill_dev(xx, a, xx + t, b, c);
    }

    /// 1 pixel frame inside `r`.
    pub fn frame(&mut self, r: Rect, c: u32) {
        if r.w <= 0 || r.h <= 0 {
            return;
        }
        let t = self.px1();
        let (x0, y0, x1, y1) = (self.dx(r.x), self.dy(r.y), self.dx(r.right()), self.dy(r.bottom()));
        self.fill_dev(x0, y0, x1, y0 + t, c);
        self.fill_dev(x0, y1 - t, x1, y1, c);
        self.fill_dev(x0, y0 + t, x0 + t, y1 - t, c);
        self.fill_dev(x1 - t, y0 + t, x1, y1 - t, c);
    }

    /// 3D edge: `tl` on the top and left, `br` on the bottom and right.
    pub fn edge(&mut self, r: Rect, tl: u32, br: u32) {
        let t = self.px1();
        let (x0, y0, x1, y1) = (self.dx(r.x), self.dy(r.y), self.dx(r.right()), self.dy(r.bottom()));
        self.fill_dev(x0, y0, x1 - t, y0 + t, tl);
        self.fill_dev(x0, y0 + t, x0 + t, y1 - t, tl);
        self.fill_dev(x0, y1 - t, x1, y1, br);
        self.fill_dev(x1 - t, y0, x1, y1 - t, br);
    }

    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: u32) {
        let t = self.px1();
        let (mut x, mut y, ex, ey) = (self.dx(x0), self.dy(y0), self.dx(x1), self.dy(y1));
        let dx = (ex - x).abs();
        let dy = -(ey - y).abs();
        let sx = if x < ex { 1 } else { -1 };
        let sy = if y < ey { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            self.fill_dev(x, y, x + t, y + t, c);
            if x == ex && y == ey {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// Anti-aliased line in logical float coordinates.
    pub fn line_aa(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, width: f32, c: u32) {
        let s = self.scale;
        let (ax, ay) = ((self.ox as f32 + x0) * s, (self.oy as f32 + y0) * s);
        let (bx, by) = ((self.ox as f32 + x1) * s, (self.oy as f32 + y1) * s);
        let hw = (width * s * 0.5).max(0.5);
        let minx = (ax.min(bx) - hw - 1.0).floor() as i32;
        let maxx = (ax.max(bx) + hw + 1.0).ceil() as i32;
        let miny = (ay.min(by) - hw - 1.0).floor() as i32;
        let maxy = (ay.max(by) + hw + 1.0).ceil() as i32;
        let (vx, vy) = (bx - ax, by - ay);
        let len2 = (vx * vx + vy * vy).max(1e-6);
        for y in miny..=maxy {
            for x in minx..=maxx {
                let (px, py) = (x as f32 + 0.5 - ax, y as f32 + 0.5 - ay);
                let t = ((px * vx + py * vy) / len2).clamp(0.0, 1.0);
                let (dx, dy) = (px - vx * t, py - vy * t);
                let d = (dx * dx + dy * dy).sqrt();
                let a = (hw + 0.5 - d).clamp(0.0, 1.0);
                if a > 0.0 {
                    self.put_dev(x, y, c, (a * 255.0) as u32);
                }
            }
        }
    }

    /// Vertical gradient from `top` to `bottom`.
    pub fn gradient(&mut self, r: Rect, top: u32, bottom: u32) {
        let (x0, y0, x1, y1) = (self.dx(r.x), self.dy(r.y), self.dx(r.right()), self.dy(r.bottom()));
        let n = (y1 - y0).max(1);
        for y in y0..y1 {
            let c = mix(top, bottom, (y - y0) as f32 / (n - 1).max(1) as f32);
            self.fill_dev(x0, y, x1, y + 1, c);
        }
    }

    /// Horizontal gradient from `left` to `right`.
    pub fn gradient_h(&mut self, r: Rect, left: u32, right: u32) {
        let (x0, y0, x1, y1) = (self.dx(r.x), self.dy(r.y), self.dx(r.right()), self.dy(r.bottom()));
        let n = (x1 - x0).max(1);
        for x in x0..x1 {
            let c = mix(left, right, (x - x0) as f32 / (n - 1).max(1) as f32);
            self.fill_dev(x, y0, x + 1, y1, c);
        }
    }

    /// Coverage of a rounded rectangle at device pixel centre (for AA).
    fn rr_cov(px: f32, py: f32, x0: f32, y0: f32, x1: f32, y1: f32, rad: f32) -> f32 {
        let cx = px.clamp(x0 + rad, x1 - rad);
        let cy = py.clamp(y0 + rad, y1 - rad);
        let (dx, dy) = (px - cx, py - cy);
        let d = (dx * dx + dy * dy).sqrt();
        if d == 0.0 {
            let inside = (px - x0).min(x1 - px).min(py - y0).min(y1 - py);
            return inside.clamp(0.0, 1.0);
        }
        (rad - d + 0.5).clamp(0.0, 1.0)
    }

    /// Rounded rectangle with a vertical gradient fill and an optional
    /// border (anti-aliased corners).
    pub fn round_rect(&mut self, r: Rect, radius: f32, top: u32, bottom: u32, border: Option<u32>) {
        if r.is_empty() {
            return;
        }
        let (x0, y0, x1, y1) = (self.dx(r.x), self.dy(r.y), self.dx(r.right()), self.dy(r.bottom()));
        let rad = (radius * self.scale).max(0.0);
        let (fx0, fy0, fx1, fy1) = (x0 as f32, y0 as f32, x1 as f32, y1 as f32);
        let bw = self.px1() as f32;
        let n = (y1 - y0).max(2);
        for y in y0..y1 {
            let fill = mix(top, bottom, (y - y0) as f32 / (n - 1) as f32);
            for x in x0..x1 {
                let (pcx, pcy) = (x as f32 + 0.5, y as f32 + 0.5);
                // fast path: interior far from the corners
                let outer = if pcx > fx0 + rad + 1.0 && pcx < fx1 - rad - 1.0 || pcy > fy0 + rad + 1.0 && pcy < fy1 - rad - 1.0 {
                    1.0
                } else {
                    Self::rr_cov(pcx, pcy, fx0, fy0, fx1, fy1, rad)
                };
                if outer <= 0.0 {
                    continue;
                }
                match border {
                    Some(bc) => {
                        let inner = Self::rr_cov(pcx, pcy, fx0 + bw, fy0 + bw, fx1 - bw, fy1 - bw, (rad - bw).max(0.0));
                        // border ring, then the fill inside
                        self.put_dev(x, y, bc, (outer * 255.0) as u32);
                        if inner > 0.0 {
                            self.put_dev(x, y, fill, (inner * 255.0) as u32);
                        }
                    }
                    None => self.put_dev(x, y, fill, (outer * 255.0) as u32),
                }
            }
        }
    }

    /// Filled ellipse (anti-aliased) with an optional border.
    pub fn ellipse(&mut self, r: Rect, fill: Option<u32>, border: Option<u32>) {
        let (x0, y0, x1, y1) = (self.dx(r.x) as f32, self.dy(r.y) as f32, self.dx(r.right()) as f32, self.dy(r.bottom()) as f32);
        let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
        let (rx, ry) = (((x1 - x0) / 2.0).max(0.5), ((y1 - y0) / 2.0).max(0.5));
        let bw = self.px1() as f32;
        for y in y0 as i32..y1 as i32 {
            for x in x0 as i32..x1 as i32 {
                let (dx, dy) = ((x as f32 + 0.5 - cx) / rx, (y as f32 + 0.5 - cy) / ry);
                let d = (dx * dx + dy * dy).sqrt();
                // distance to the edge in pixels (approx.)
                let edge = (1.0 - d) * rx.min(ry);
                let outer = (edge + 0.5).clamp(0.0, 1.0);
                if outer <= 0.0 {
                    continue;
                }
                match border {
                    Some(bc) => {
                        self.put_dev(x, y, bc, (outer * 255.0) as u32);
                        if let Some(f) = fill {
                            let inner = (edge - bw + 0.5).clamp(0.0, 1.0);
                            if inner > 0.0 {
                                self.put_dev(x, y, f, (inner * 255.0) as u32);
                            }
                        }
                    }
                    None => {
                        if let Some(f) = fill {
                            self.put_dev(x, y, f, (outer * 255.0) as u32)
                        }
                    }
                }
            }
        }
    }

    /// Filled polygon (logical coordinates, anti-aliased by 4x4 sampling).
    pub fn polygon(&mut self, pts: &[(f32, f32)], c: u32) {
        if pts.len() < 3 {
            return;
        }
        let s = self.scale;
        let p: Vec<(f32, f32)> = pts.iter().map(|&(x, y)| ((self.ox as f32 + x) * s, (self.oy as f32 + y) * s)).collect();
        let minx = p.iter().map(|q| q.0).fold(f32::MAX, f32::min).floor() as i32;
        let maxx = p.iter().map(|q| q.0).fold(f32::MIN, f32::max).ceil() as i32;
        let miny = p.iter().map(|q| q.1).fold(f32::MAX, f32::min).floor() as i32;
        let maxy = p.iter().map(|q| q.1).fold(f32::MIN, f32::max).ceil() as i32;
        let inside = |x: f32, y: f32| {
            let mut c = false;
            let mut j = p.len() - 1;
            for i in 0..p.len() {
                let (xi, yi) = p[i];
                let (xj, yj) = p[j];
                if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
                    c = !c;
                }
                j = i;
            }
            c
        };
        for y in miny..maxy {
            for x in minx..maxx {
                let mut n = 0;
                for sy in 0..4 {
                    for sx in 0..4 {
                        if inside(x as f32 + (sx as f32 + 0.5) / 4.0, y as f32 + (sy as f32 + 0.5) / 4.0) {
                            n += 1;
                        }
                    }
                }
                if n > 0 {
                    self.put_dev(x, y, c, n * 255 / 16);
                }
            }
        }
    }

    // ---- text

    pub fn font_px(&self, f: &Font) -> f32 {
        f.em() * self.scale
    }

    /// Width of a single line in logical pixels.
    pub fn text_width(&self, f: &Font, s: &str) -> i32 {
        (font::text_width(f.face(), self.font_px(f), s) / self.scale).ceil() as i32
    }

    /// Draws one line with the text cell's top left at `x`, `y` (logical).
    pub fn text(&mut self, x: i32, y: i32, s: &str, f: &Font, color: u32) {
        let px = self.font_px(f);
        let face = f.face();
        let (asc, _) = font::metrics(face, px);
        // centre the font's ascent+descent in the VCL line height
        let lh = line_height(f) as f32 * self.scale;
        let (a, d) = font::metrics(face, px);
        let top = self.dy(y) as f32 + ((lh - (a + d)) / 2.0).max(0.0);
        let base = (top + asc).round() as i32;
        let mut pen = self.dx(x) as f32;
        let x_start = pen;
        let italic = f.italic;
        for ch in s.chars() {
            let g = font::glyph(face, px, ch);
            let gx = pen.round() as i32 + g.ox;
            let gy = base + g.oy;
            if gx > self.clip[2] {
                break;
            }
            for row in 0..g.h {
                let shear = if italic { ((g.h - row) as f32 * 0.2) as i32 } else { 0 };
                for col in 0..g.w {
                    let cv = g.cov[row * g.w + col] as u32;
                    if cv > 0 {
                        // slight gamma lift so that small text stays crisp
                        let a = ((cv as f32 / 255.0).powf(0.8) * 255.0) as u32;
                        self.put_dev(gx + col as i32 + shear, gy + row as i32, color, a);
                    }
                }
            }
            if f.bold && face != font::Face::SansBold {
                // synthetic bold for faces without a bold variant
                for row in 0..g.h {
                    for col in 0..g.w {
                        let cv = g.cov[row * g.w + col] as u32;
                        if cv > 0 {
                            self.put_dev(gx + col as i32 + 1, gy + row as i32, color, cv);
                        }
                    }
                }
            }
            pen += g.advance;
        }
        if f.underline {
            let t = self.px1();
            self.fill_dev(x_start as i32, base + t, pen.round() as i32, base + 2 * t, color);
        }
    }

    /// Splits text into lines (CR/LF and, with `wrap`, word wrapping to
    /// `width` logical pixels).
    pub fn layout_lines(&self, s: &str, f: &Font, width: i32, wrap: bool) -> Vec<String> {
        let mut out = Vec::new();
        for para in s.replace("\r\n", "\n").replace('\r', "\n").split('\n') {
            if !wrap || self.text_width(f, para) <= width {
                out.push(para.to_string());
                continue;
            }
            let mut line = String::new();
            for word in para.split(' ') {
                let cand = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
                if self.text_width(f, &cand) <= width || line.is_empty() {
                    line = cand;
                } else {
                    out.push(std::mem::take(&mut line));
                    line = word.to_string();
                }
            }
            out.push(line);
        }
        out
    }

    /// Text in a rectangle with alignment and optional word wrap.
    #[allow(clippy::too_many_arguments)]
    pub fn text_in(&mut self, r: Rect, s: &str, f: &Font, color: u32, ha: HAlign, va: VAlign, wrap: bool) {
        let lines = self.layout_lines(s, f, r.w, wrap);
        let lh = line_height(f);
        let total = lh * lines.len() as i32;
        let mut y = match va {
            VAlign::Top => r.y,
            VAlign::Center => r.y + (r.h - total) / 2,
            VAlign::Bottom => r.bottom() - total,
        };
        for l in &lines {
            let w = self.text_width(f, l);
            let x = match ha {
                HAlign::Left => r.x,
                HAlign::Center => r.x + (r.w - w) / 2,
                HAlign::Right => r.right() - w,
            };
            self.text(x, y, l, f, color);
            y += lh;
        }
    }

    /// The least change that fits a caption into `r`: a text too wide for it
    /// is drawn in a smaller font (down to 8 pixels), and what still does
    /// not fit ends in "..." (one-line texts; wrapped text is left to the
    /// caller's clipping).  Translated captions are often longer than the
    /// English the windows were laid out for.
    pub fn fit(&self, r: Rect, s: &str, f: &Font, wrap: bool) -> (Font, String) {
        if self.fits(r, s, f, wrap) {
            return (f.clone(), s.to_string());
        }
        let mut font = f.clone();
        let mut h = f.em().round() as i32 - 1;
        while h >= 8 {
            font.height = -h;
            if self.fits(r, s, &font, wrap) {
                return (font, s.to_string());
            }
            h -= 1;
        }
        if wrap || s.contains('\n') {
            return (font, s.to_string());
        }
        let short = self.elide(s, &font, r.w);
        (font, short)
    }

    /// Whether `s` (wrapped to `r.w` when `wrap`) fits into `r`; a line may
    /// stand over by [`SLACK`] pixels, as the English captions of the form
    /// files do here and there.
    fn fits(&self, r: Rect, s: &str, f: &Font, wrap: bool) -> bool {
        let lines = self.layout_lines(s, f, r.w, wrap);
        lines.iter().all(|l| self.text_width(f, l) <= r.w + SLACK) && (!wrap || lines.len() as i32 * line_height(f) <= r.h)
    }

    /// One line shortened with "..." to `w` pixels.
    fn elide(&self, s: &str, f: &Font, w: i32) -> String {
        let mut t: Vec<char> = s.chars().collect();
        while !t.is_empty() {
            t.pop();
            let cand = format!("{}...", t.iter().collect::<String>().trim_end());
            if self.text_width(f, &cand) <= w {
                return cand;
            }
        }
        String::new()
    }

    /// [`text_in`](Canvas::text_in) with the text made to fit (see [`fit`](Canvas::fit)).
    #[allow(clippy::too_many_arguments)]
    pub fn text_fit(&mut self, r: Rect, s: &str, f: &Font, color: u32, ha: HAlign, va: VAlign, wrap: bool) {
        let (font, text) = self.fit(r, s, f, wrap);
        if wrap && !self.fits(r, &text, &font, true) {
            // the smallest font still has a word too long for the width, or
            // too many lines: what does not fit goes, line by line
            let max_lines = (r.h / line_height(&font)).max(1) as usize;
            let mut lines = self.layout_lines(&text, &font, r.w, true);
            lines.truncate(max_lines);
            let lines: Vec<String> = lines.iter().map(|l| if self.text_width(&font, l) > r.w { self.elide(l, &font, r.w) } else { l.clone() }).collect();
            self.text_in(r, &lines.join("\n"), &font, color, ha, va, false);
        } else {
            self.text_in(r, &text, &font, color, ha, va, wrap);
        }
    }

    // ---- images

    /// Draws a bitmap at logical `x`, `y`, scaled by the canvas scale.
    pub fn image(&mut self, x: i32, y: i32, b: &Bitmap) {
        self.image_stretch(Rect::new(x, y, b.w as i32, b.h as i32), b, false);
    }

    /// Draws a bitmap into a logical rectangle (nearest neighbour or
    /// bilinear with `smooth`).
    pub fn image_stretch(&mut self, r: Rect, b: &Bitmap, smooth: bool) {
        if b.is_empty() || r.is_empty() {
            return;
        }
        let (x0, y0, x1, y1) = (self.dx(r.x), self.dy(r.y), self.dx(r.right()), self.dy(r.bottom()));
        self.image_dev(x0, y0, x1 - x0, y1 - y0, b, smooth);
    }

    /// Draws a bitmap into device pixels.
    pub fn image_dev(&mut self, x0: i32, y0: i32, w: i32, h: i32, b: &Bitmap, smooth: bool) {
        if w <= 0 || h <= 0 || b.is_empty() {
            return;
        }
        let cx0 = x0.max(self.clip[0]);
        let cy0 = y0.max(self.clip[1]);
        let cx1 = (x0 + w).min(self.clip[2]);
        let cy1 = (y0 + h).min(self.clip[3]);
        let one = w as usize == b.w && h as usize == b.h;
        for y in cy0..cy1 {
            let sy = (y - y0) as f32 * b.h as f32 / h as f32;
            for x in cx0..cx1 {
                let p = if one {
                    b.px[(y - y0) as usize * b.w + (x - x0) as usize]
                } else if smooth && (w as usize) < b.w {
                    // box filter when reducing
                    let sx0 = (x - x0) as usize * b.w / w as usize;
                    let sx1 = ((x - x0 + 1) as usize * b.w / w as usize).max(sx0 + 1).min(b.w);
                    let sy0 = (y - y0) as usize * b.h / h as usize;
                    let sy1 = ((y - y0 + 1) as usize * b.h / h as usize).max(sy0 + 1).min(b.h);
                    let (mut r, mut g, mut bb, mut a, mut n) = (0u32, 0u32, 0u32, 0u32, 0u32);
                    for yy in sy0..sy1 {
                        for xx in sx0..sx1 {
                            let q = b.px[yy * b.w + xx];
                            a += q >> 24;
                            r += q >> 16 & 255;
                            g += q >> 8 & 255;
                            bb += q & 255;
                            n += 1;
                        }
                    }
                    (a / n) << 24 | (r / n) << 16 | (g / n) << 8 | bb / n
                } else {
                    let sx = ((x - x0) as f32 * b.w as f32 / w as f32) as usize;
                    b.px[(sy as usize).min(b.h - 1) * b.w + sx.min(b.w - 1)]
                };
                let a = p >> 24;
                if a == 255 {
                    self.buf[(y * self.bw + x) as usize] = p & 0xFF_FFFF;
                } else if a > 0 {
                    let d = &mut self.buf[(y * self.bw + x) as usize];
                    *d = blend(*d, p, a);
                }
            }
        }
    }
}
