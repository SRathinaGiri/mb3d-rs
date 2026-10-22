//! Minimal image decoders (std only) for MB3D's maps and background
//! pictures: PNG (all colour types, 1–16 bit, Adam7), JPEG (baseline and
//! progressive, any chroma subsampling, grey or YCbCr), BMP (8/24/32 bit)
//! and binary PGM (8/16 bit).

/// A decoded image.  `data` holds RGB triples row by row; with `deep` the
/// values are 16 bit (0..65535), otherwise 8 bit values (0..255).
#[derive(Clone, Debug)]
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub deep: bool,
    pub data: Vec<[u16; 3]>,
}

impl Image {
    #[inline]
    pub fn pixel(&self, x: usize, y: usize) -> [u16; 3] {
        self.data[y * self.width + x]
    }
}

/// Decodes an image file by its contents.
pub fn decode(bytes: &[u8]) -> Result<Image, String> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        decode_png(bytes)
    } else if bytes.starts_with(&[0xFF, 0xD8]) {
        decode_jpeg(bytes)
    } else if bytes.starts_with(b"BM") {
        decode_bmp(bytes)
    } else if bytes.starts_with(b"P5") {
        decode_pgm(bytes)
    } else {
        Err("unknown image format".into())
    }
}

/// The pixels of a `w`×`h` image, refusing sizes no picture file has.
fn pixels(w: usize, h: usize) -> Result<Vec<[u16; 3]>, String> {
    match w.checked_mul(h) {
        Some(n) if n <= 1 << 28 => Ok(vec![[0u16; 3]; n]),
        _ => Err(format!("image size {w}x{h} is too large")),
    }
}

pub fn load(path: &std::path::Path) -> Result<Image, String> {
    let b = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    decode(&b).map_err(|e| format!("{}: {e}", path.display()))
}

// ---------------------------------------------------------------------------
// inflate (RFC 1951) and zlib
// ---------------------------------------------------------------------------

struct BitReader<'a> {
    d: &'a [u8],
    pos: usize,
    bit: u32,
    nbits: u32,
}

impl<'a> BitReader<'a> {
    fn new(d: &'a [u8]) -> Self {
        BitReader { d, pos: 0, bit: 0, nbits: 0 }
    }
    #[inline]
    fn need(&mut self, n: u32) -> Result<(), String> {
        while self.nbits < n {
            let b = *self.d.get(self.pos).ok_or("inflate: unexpected end of data")?;
            self.pos += 1;
            self.bit |= (b as u32) << self.nbits;
            self.nbits += 8;
        }
        Ok(())
    }
    #[inline]
    fn bits(&mut self, n: u32) -> Result<u32, String> {
        if n == 0 {
            return Ok(0);
        }
        self.need(n)?;
        let v = self.bit & ((1u32 << n) - 1);
        self.bit >>= n;
        self.nbits -= n;
        Ok(v)
    }
    fn align(&mut self) {
        self.bit = 0;
        self.nbits = 0;
    }
}

/// Canonical Huffman decoding table (counts per length, sorted symbols).
struct Huff {
    counts: [u16; 16],
    symbols: Vec<u16>,
}

impl Huff {
    fn new(lengths: &[u8]) -> Self {
        let mut counts = [0u16; 16];
        for &l in lengths {
            counts[l as usize] += 1;
        }
        counts[0] = 0;
        let mut offs = [0u16; 16];
        for i in 1..16 {
            offs[i] = offs[i - 1] + counts[i - 1];
        }
        let mut symbols = vec![0u16; lengths.len()];
        for (s, &l) in lengths.iter().enumerate() {
            if l != 0 {
                symbols[offs[l as usize] as usize] = s as u16;
                offs[l as usize] += 1;
            }
        }
        Huff { counts, symbols }
    }
    fn decode(&self, br: &mut BitReader) -> Result<u16, String> {
        let (mut code, mut first, mut index) = (0i32, 0i32, 0i32);
        for len in 1..16 {
            code |= br.bits(1)? as i32;
            let count = self.counts[len] as i32;
            if code - count < first {
                return Ok(self.symbols[(index + (code - first)) as usize]);
            }
            index += count;
            first += count;
            first <<= 1;
            code <<= 1;
        }
        Err("inflate: bad Huffman code".into())
    }
}

const LBASE: [u16; 29] = [3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258];
const LEXT: [u8; 29] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0];
const DBASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DEXT: [u8; 30] = [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13];

pub fn inflate(d: &[u8]) -> Result<Vec<u8>, String> {
    let mut out: Vec<u8> = Vec::with_capacity(d.len() * 4);
    let mut br = BitReader::new(d);
    loop {
        let last = br.bits(1)?;
        let ty = br.bits(2)?;
        match ty {
            0 => {
                br.align();
                let p = br.pos;
                if p + 4 > d.len() {
                    return Err("inflate: truncated stored block".into());
                }
                let len = u16::from_le_bytes([d[p], d[p + 1]]) as usize;
                let s = p + 4;
                out.extend_from_slice(d.get(s..s + len).ok_or("inflate: truncated stored block")?);
                br.pos = s + len;
            }
            1 | 2 => {
                let (lit, dist) = if ty == 1 {
                    let mut l = [0u8; 288];
                    for (i, v) in l.iter_mut().enumerate() {
                        *v = match i {
                            0..=143 => 8,
                            144..=255 => 9,
                            256..=279 => 7,
                            _ => 8,
                        };
                    }
                    (Huff::new(&l), Huff::new(&[5u8; 30]))
                } else {
                    let hlit = br.bits(5)? as usize + 257;
                    let hdist = br.bits(5)? as usize + 1;
                    let hclen = br.bits(4)? as usize + 4;
                    const ORD: [usize; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];
                    let mut cl = [0u8; 19];
                    for &o in ORD.iter().take(hclen) {
                        cl[o] = br.bits(3)? as u8;
                    }
                    let ch = Huff::new(&cl);
                    let mut lens = vec![0u8; hlit + hdist];
                    let mut i = 0;
                    while i < hlit + hdist {
                        let sym = ch.decode(&mut br)?;
                        match sym {
                            0..=15 => {
                                lens[i] = sym as u8;
                                i += 1;
                            }
                            16 => {
                                let prev = *lens.get(i.wrapping_sub(1)).ok_or("inflate: bad repeat")?;
                                for _ in 0..3 + br.bits(2)? {
                                    *lens.get_mut(i).ok_or("inflate: bad lengths")? = prev;
                                    i += 1;
                                }
                            }
                            17 => i += 3 + br.bits(3)? as usize,
                            _ => i += 11 + br.bits(7)? as usize,
                        }
                    }
                    if i > hlit + hdist {
                        return Err("inflate: bad code lengths".into());
                    }
                    (Huff::new(&lens[..hlit]), Huff::new(&lens[hlit..]))
                };
                loop {
                    let sym = lit.decode(&mut br)? as usize;
                    if sym < 256 {
                        out.push(sym as u8);
                    } else if sym == 256 {
                        break;
                    } else {
                        let k = sym - 257;
                        if k >= 29 {
                            return Err("inflate: bad length".into());
                        }
                        let len = LBASE[k] as usize + br.bits(LEXT[k] as u32)? as usize;
                        let ds = dist.decode(&mut br)? as usize;
                        if ds >= 30 {
                            return Err("inflate: bad distance".into());
                        }
                        let dd = DBASE[ds] as usize + br.bits(DEXT[ds] as u32)? as usize;
                        if dd > out.len() {
                            return Err("inflate: distance too far back".into());
                        }
                        let start = out.len() - dd;
                        for j in 0..len {
                            let b = out[start + j];
                            out.push(b);
                        }
                    }
                }
            }
            _ => return Err("inflate: bad block type".into()),
        }
        if last == 1 {
            break;
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// PNG
// ---------------------------------------------------------------------------

fn decode_png(b: &[u8]) -> Result<Image, String> {
    let mut p = 8;
    let (mut w, mut h, mut depth, mut ctype, mut interlace) = (0usize, 0usize, 0u8, 0u8, 0u8);
    let mut pal: Vec<[u8; 3]> = Vec::new();
    let mut idat = Vec::new();
    while p + 8 <= b.len() {
        let len = u32::from_be_bytes(b[p..p + 4].try_into().unwrap()) as usize;
        let ty = &b[p + 4..p + 8];
        let data = b.get(p + 8..p + 8 + len).ok_or("png: truncated chunk")?;
        match ty {
            b"IHDR" => {
                if data.len() < 13 {
                    return Err("png: bad header".into());
                }
                w = u32::from_be_bytes(data[0..4].try_into().unwrap()) as usize;
                h = u32::from_be_bytes(data[4..8].try_into().unwrap()) as usize;
                depth = data[8];
                ctype = data[9];
                interlace = data[12];
            }
            b"PLTE" => pal = data.chunks(3).filter(|c| c.len() == 3).map(|c| [c[0], c[1], c[2]]).collect(),
            b"IDAT" => idat.extend_from_slice(data),
            b"IEND" => break,
            _ => {}
        }
        p += 12 + len;
    }
    if w == 0 || h == 0 || idat.len() < 2 {
        return Err("png: no image data".into());
    }
    if ![1, 2, 4, 8, 16].contains(&depth) {
        return Err(format!("png: bit depth {depth}"));
    }
    let data = pixels(w, h)?;
    let raw = inflate(&idat[2..])?;
    let channels = match ctype {
        0 => 1,
        2 => 3,
        3 => 1,
        4 => 2,
        6 => 4,
        _ => return Err(format!("png: colour type {ctype}")),
    };
    let bpp_bits = channels * depth as usize;
    let bpp = bpp_bits.div_ceil(8).max(1);
    let deep = depth == 16 && ctype != 3;
    let mut img = Image { width: w, height: h, deep, data };
    // (x0, y0, dx, dy) of the Adam7 passes, or one pass
    let passes: Vec<(usize, usize, usize, usize)> = if interlace == 1 {
        vec![(0, 0, 8, 8), (4, 0, 8, 8), (0, 4, 4, 8), (2, 0, 4, 4), (0, 2, 2, 4), (1, 0, 2, 2), (0, 1, 1, 2)]
    } else {
        vec![(0, 0, 1, 1)]
    };
    let mut pos = 0;
    for (x0, y0, dx, dy) in passes {
        if x0 >= w || y0 >= h {
            continue;
        }
        let pw = (w - x0).div_ceil(dx);
        let ph = (h - y0).div_ceil(dy);
        let stride = (pw * bpp_bits).div_ceil(8);
        let mut prev = vec![0u8; stride];
        let mut cur = vec![0u8; stride];
        for py in 0..ph {
            let f = *raw.get(pos).ok_or("png: truncated image data")?;
            let line = raw.get(pos + 1..pos + 1 + stride).ok_or("png: truncated image data")?;
            pos += 1 + stride;
            for i in 0..stride {
                let a = if i >= bpp { cur[i - bpp] as i32 } else { 0 };
                let up = prev[i] as i32;
                let c = if i >= bpp { prev[i - bpp] as i32 } else { 0 };
                let x = line[i] as i32;
                cur[i] = match f {
                    0 => x,
                    1 => x + a,
                    2 => x + up,
                    3 => x + (a + up) / 2,
                    4 => {
                        let pp = a + up - c;
                        let (pa, pb, pc) = ((pp - a).abs(), (pp - up).abs(), (pp - c).abs());
                        x + if pa <= pb && pa <= pc {
                            a
                        } else if pb <= pc {
                            up
                        } else {
                            c
                        }
                    }
                    _ => return Err("png: bad filter".into()),
                } as u8;
            }
            let sample = |idx: usize| -> u16 {
                match depth {
                    16 => u16::from_be_bytes([cur[idx * 2], cur[idx * 2 + 1]]),
                    8 => cur[idx] as u16,
                    _ => {
                        let bit = idx * depth as usize;
                        let v = (cur[bit / 8] >> (8 - depth as usize - bit % 8)) & ((1u8 << depth) - 1);
                        v as u16
                    }
                }
            };
            let scale = |v: u16| -> u16 {
                match depth {
                    16 | 8 => v,
                    d => (v as u32 * 255 / ((1u32 << d) - 1)) as u16,
                }
            };
            for px in 0..pw {
                let rgb = match ctype {
                    3 => {
                        let c = pal.get(sample(px) as usize).copied().unwrap_or([0, 0, 0]);
                        [c[0] as u16, c[1] as u16, c[2] as u16]
                    }
                    0 | 4 => {
                        let v = scale(sample(px * channels));
                        [v, v, v]
                    }
                    _ => [sample(px * channels), sample(px * channels + 1), sample(px * channels + 2)],
                };
                img.data[(y0 + py * dy) * w + x0 + px * dx] = rgb;
            }
            std::mem::swap(&mut prev, &mut cur);
        }
    }
    Ok(img)
}

// ---------------------------------------------------------------------------
// BMP, PGM
// ---------------------------------------------------------------------------

fn decode_bmp(b: &[u8]) -> Result<Image, String> {
    let u32le = |o: usize| -> Result<u32, String> {
        Ok(u32::from_le_bytes(b.get(o..o + 4).ok_or("bmp: truncated")?.try_into().unwrap()))
    };
    let off = u32le(10)? as usize;
    let hsize = u32le(14)? as usize;
    let w = u32le(18)? as i32;
    let hh = u32le(22)? as i32;
    let bits = u16::from_le_bytes(b.get(28..30).ok_or("bmp: truncated")?.try_into().unwrap());
    let comp = u32le(30)?;
    if comp != 0 && comp != 3 {
        return Err("bmp: compressed bitmaps are not supported".into());
    }
    let (w, h, flip) = (w.unsigned_abs() as usize, hh.unsigned_abs() as usize, hh > 0);
    let data = pixels(w, h)?;
    let stride = (w * bits as usize).div_ceil(32) * 4;
    let pal_off = 14 + hsize;
    let mut img = Image { width: w, height: h, deep: false, data };
    for y in 0..h {
        let row = off + y * stride;
        let ty = if flip { h - 1 - y } else { y };
        for x in 0..w {
            let c = match bits {
                24 | 32 => {
                    let p = row + x * (bits as usize / 8);
                    let s = b.get(p..p + 3).ok_or("bmp: truncated")?;
                    [s[2], s[1], s[0]]
                }
                8 => {
                    let i = *b.get(row + x).ok_or("bmp: truncated")? as usize;
                    let p = pal_off + i * 4;
                    let s = b.get(p..p + 3).ok_or("bmp: bad palette")?;
                    [s[2], s[1], s[0]]
                }
                _ => return Err(format!("bmp: {bits} bit images are not supported")),
            };
            img.data[ty * w + x] = [c[0] as u16, c[1] as u16, c[2] as u16];
        }
    }
    Ok(img)
}

fn decode_pgm(b: &[u8]) -> Result<Image, String> {
    // header: P5 <w> <h> <max> (whitespace and comments)
    let mut fields = Vec::new();
    let mut p = 2;
    while fields.len() < 3 {
        while p < b.len() && (b[p].is_ascii_whitespace() || b[p] == b'#') {
            if b[p] == b'#' {
                while p < b.len() && b[p] != b'\n' {
                    p += 1;
                }
            } else {
                p += 1;
            }
        }
        let s = p;
        while p < b.len() && b[p].is_ascii_digit() {
            p += 1;
        }
        fields.push(std::str::from_utf8(&b[s..p]).ok().and_then(|t| t.parse::<usize>().ok()).ok_or("pgm: bad header")?);
    }
    p += 1;
    let (w, h, max) = (fields[0], fields[1], fields[2]);
    if !(1..=65535).contains(&max) {
        return Err(format!("pgm: maximum value {max}"));
    }
    let deep = max > 255;
    let mut img = Image { width: w, height: h, deep, data: pixels(w, h)? };
    // samples scaled from 0..max to 0..255 or 0..65535
    let full = if deep { 65535 } else { 255 };
    for (i, px) in img.data.iter_mut().enumerate() {
        let v = if deep {
            let o = p + i * 2;
            u16::from_be_bytes([*b.get(o).ok_or("pgm: truncated")?, *b.get(o + 1).ok_or("pgm: truncated")?])
        } else {
            *b.get(p + i).ok_or("pgm: truncated")? as u16
        };
        let v = (v.min(max as u16) as u32 * full / max as u32) as u16;
        *px = [v, v, v];
    }
    Ok(img)
}

// ---------------------------------------------------------------------------
// JPEG (baseline + progressive)
// ---------------------------------------------------------------------------

const ZIGZAG: [usize; 64] = [
    0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20, 13, 6, 7, 14, 21, 28, 35, 42, 49, 56, 57,
    50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59, 52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63,
];

#[derive(Clone, Default)]
struct JHuff {
    /// (length, code) -> symbol via maxcode/valptr tables
    maxcode: [i32; 18],
    valptr: [i32; 17],
    mincode: [i32; 17],
    vals: Vec<u8>,
}

impl JHuff {
    fn new(counts: &[u8; 16], vals: Vec<u8>) -> Self {
        let mut h = JHuff { vals, ..Default::default() };
        let mut code = 0i32;
        let mut k = 0i32;
        for l in 1..=16 {
            let n = counts[l - 1] as i32;
            h.valptr[l] = k;
            h.mincode[l] = code;
            code += n;
            k += n;
            h.maxcode[l] = if n > 0 { code - 1 } else { -1 };
            code <<= 1;
        }
        h.maxcode[17] = i32::MAX;
        h
    }
}

struct JBits<'a> {
    d: &'a [u8],
    pos: usize,
    acc: u32,
    n: u32,
    /// a marker was reached; zeros are fed from now on
    marker: bool,
}

impl<'a> JBits<'a> {
    fn bit(&mut self) -> u32 {
        if self.n == 0 {
            let mut b = 0u8;
            if !self.marker && self.pos < self.d.len() {
                b = self.d[self.pos];
                if b == 0xFF {
                    let nx = self.d.get(self.pos + 1).copied().unwrap_or(0);
                    if nx == 0 {
                        self.pos += 2;
                    } else {
                        self.marker = true;
                        b = 0;
                    }
                } else {
                    self.pos += 1;
                }
            }
            self.acc = b as u32;
            self.n = 8;
        }
        self.n -= 1;
        (self.acc >> self.n) & 1
    }
    fn bits(&mut self, n: u32) -> u32 {
        let mut v = 0;
        for _ in 0..n {
            v = (v << 1) | self.bit();
        }
        v
    }
    fn decode(&mut self, h: &JHuff) -> Result<u8, String> {
        let mut code = 0i32;
        for l in 1..=16 {
            code = (code << 1) | self.bit() as i32;
            if code <= h.maxcode[l] {
                let i = h.valptr[l] + code - h.mincode[l];
                return h.vals.get(i as usize).copied().ok_or_else(|| "jpeg: bad Huffman code".to_string());
            }
        }
        Err("jpeg: bad Huffman code".into())
    }
    /// skips to the next restart marker
    fn restart(&mut self) {
        self.n = 0;
        self.marker = false;
        while self.pos + 1 < self.d.len() && !(self.d[self.pos] == 0xFF && (0xD0..=0xD7).contains(&self.d[self.pos + 1])) {
            self.pos += 1;
        }
        self.pos += 2;
    }
}

#[inline]
fn extend(v: u32, s: u32) -> i32 {
    if s == 0 {
        0
    } else if v < (1 << (s - 1)) {
        v as i32 - (1 << s) + 1
    } else {
        v as i32
    }
}

/// The bit count of a DC difference (at most 16 by the standard).
fn dc_size(s: u8) -> Result<u32, String> {
    if s > 16 {
        return Err("jpeg: bad DC difference".into());
    }
    Ok(s as u32)
}

struct JComp {
    id: u8,
    h: usize,
    v: usize,
    tq: usize,
    /// blocks per line / column (padded to whole MCUs)
    bw: usize,
    bh: usize,
    coefs: Vec<[i32; 64]>,
    dc_pred: i32,
}

fn decode_jpeg(b: &[u8]) -> Result<Image, String> {
    let mut qt = [[0u16; 64]; 4];
    let mut dc_t: Vec<JHuff> = vec![JHuff::default(); 4];
    let mut ac_t: Vec<JHuff> = vec![JHuff::default(); 4];
    let mut comps: Vec<JComp> = Vec::new();
    let (mut w, mut h) = (0usize, 0usize);
    let (mut hmax, mut vmax) = (1usize, 1usize);
    let (mut mcux, mut mcuy) = (0usize, 0usize);
    let mut progressive = false;
    let mut restart = 0usize;
    let mut eobrun: u32;
    let mut adobe_rgb = false;
    let mut p = 2;
    let be16 = |o: usize| -> Result<usize, String> {
        Ok(u16::from_be_bytes(b.get(o..o + 2).ok_or("jpeg: truncated")?.try_into().unwrap()) as usize)
    };
    loop {
        while p < b.len() && b[p] != 0xFF {
            p += 1;
        }
        while p < b.len() && b[p] == 0xFF {
            p += 1;
        }
        if p >= b.len() {
            break;
        }
        let m = b[p];
        p += 1;
        if m == 0xD9 {
            break;
        }
        if (0xD0..=0xD7).contains(&m) || m == 0x01 {
            continue;
        }
        let len = be16(p)?;
        let seg = b.get(p + 2..p + len).ok_or("jpeg: truncated segment")?;
        match m {
            0xDB => {
                let mut q = 0;
                while q < seg.len() {
                    let pq = seg[q] >> 4;
                    let t = (seg[q] & 3) as usize;
                    q += 1;
                    if seg.len() < q + if pq == 0 { 64 } else { 128 } {
                        return Err("jpeg: truncated quantisation table".into());
                    }
                    for k in 0..64 {
                        qt[t][ZIGZAG[k]] = if pq == 0 {
                            seg[q + k] as u16
                        } else {
                            u16::from_be_bytes([seg[q + 2 * k], seg[q + 2 * k + 1]])
                        };
                    }
                    q += if pq == 0 { 64 } else { 128 };
                }
            }
            0xC4 => {
                let mut q = 0;
                while q < seg.len() {
                    let class = seg[q] >> 4;
                    let t = (seg[q] & 3) as usize;
                    let counts: [u8; 16] = seg.get(q + 1..q + 17).ok_or("jpeg: truncated Huffman table")?.try_into().unwrap();
                    let n: usize = counts.iter().map(|&c| c as usize).sum();
                    let vals = seg.get(q + 17..q + 17 + n).ok_or("jpeg: truncated Huffman table")?.to_vec();
                    let tbl = JHuff::new(&counts, vals);
                    if class == 0 {
                        dc_t[t] = tbl;
                    } else {
                        ac_t[t] = tbl;
                    }
                    q += 17 + n;
                }
            }
            0xC0 | 0xC1 | 0xC2 => {
                progressive = m == 0xC2;
                if seg.len() < 6 || seg.len() < 6 + seg[5] as usize * 3 {
                    return Err("jpeg: truncated frame header".into());
                }
                if seg[0] != 8 {
                    return Err("jpeg: only 8 bit precision is supported".into());
                }
                h = u16::from_be_bytes([seg[1], seg[2]]) as usize;
                w = u16::from_be_bytes([seg[3], seg[4]]) as usize;
                let n = seg[5] as usize;
                if n != 1 && n != 3 {
                    return Err(format!("jpeg: {n} components are not supported"));
                }
                comps.clear();
                for i in 0..n {
                    let c = &seg[6 + i * 3..9 + i * 3];
                    comps.push(JComp {
                        id: c[0],
                        h: (c[1] >> 4).max(1) as usize,
                        v: (c[1] & 15).max(1) as usize,
                        tq: (c[2] & 3) as usize,
                        bw: 0,
                        bh: 0,
                        coefs: Vec::new(),
                        dc_pred: 0,
                    });
                }
                hmax = comps.iter().map(|c| c.h).max().unwrap();
                vmax = comps.iter().map(|c| c.v).max().unwrap();
                mcux = w.div_ceil(8 * hmax);
                mcuy = h.div_ceil(8 * vmax);
                for c in comps.iter_mut() {
                    c.bw = mcux * c.h;
                    c.bh = mcuy * c.v;
                    c.coefs = vec![[0i32; 64]; c.bw * c.bh];
                }
            }
            0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF => return Err("jpeg: lossless / arithmetic coding is not supported".into()),
            0xDD => restart = be16(p + 2)?,
            0xEE => adobe_rgb = seg.len() >= 12 && seg.starts_with(b"Adobe") && seg[11] == 0,
            0xDA => {
                if comps.is_empty() {
                    return Err("jpeg: scan before frame header".into());
                }
                let ns = *seg.first().ok_or("jpeg: truncated scan header")? as usize;
                if ns == 0 || seg.len() < 4 + ns * 2 {
                    return Err("jpeg: truncated scan header".into());
                }
                let mut sc: Vec<(usize, usize, usize)> = Vec::new(); // comp index, dc table, ac table
                for i in 0..ns {
                    let cid = seg[1 + i * 2];
                    let t = seg[2 + i * 2];
                    let ci = comps.iter().position(|c| c.id == cid).ok_or("jpeg: bad scan component")?;
                    sc.push((ci, (t >> 4) as usize & 3, (t & 3) as usize));
                }
                let o = 1 + ns * 2;
                let (ss, se, ah, al) = (seg[o] as usize, seg[o + 1] as usize, (seg[o + 2] >> 4) as u32, (seg[o + 2] & 15) as u32);
                if progressive && (ss > se || se > 63) {
                    return Err("jpeg: bad spectral selection".into());
                }
                let data = &b[p + len..];
                let mut br = JBits { d: data, pos: 0, acc: 0, n: 0, marker: false };
                for c in comps.iter_mut() {
                    c.dc_pred = 0;
                }
                eobrun = 0;
                let single = ns == 1;
                // number of "units" (MCUs, or blocks for a single component scan)
                let (ux, uy) = if single {
                    let c = &comps[sc[0].0];
                    ((w * c.h).div_ceil(8 * hmax), (h * c.v).div_ceil(8 * vmax))
                } else {
                    (mcux, mcuy)
                };
                let mut count = 0usize;
                for uyy in 0..uy {
                    for uxx in 0..ux {
                        if restart > 0 && count > 0 && count % restart == 0 {
                            br.restart();
                            for c in comps.iter_mut() {
                                c.dc_pred = 0;
                            }
                            eobrun = 0;
                        }
                        count += 1;
                        for &(ci, td, ta) in &sc {
                            let (bh_, bv_) = if single { (1, 1) } else { (comps[ci].h, comps[ci].v) };
                            for by in 0..bv_ {
                                for bx in 0..bh_ {
                                    let (gx, gy) = if single { (uxx, uyy) } else { (uxx * comps[ci].h + bx, uyy * comps[ci].v + by) };
                                    let c = &mut comps[ci];
                                    let idx = gy * c.bw + gx;
                                    let blk = &mut c.coefs[idx];
                                    if !progressive {
                                        // baseline: whole block
                                        let s = dc_size(br.decode(&dc_t[td])?)?;
                                        let diff = extend(br.bits(s), s);
                                        c.dc_pred = c.dc_pred.wrapping_add(diff);
                                        blk[0] = c.dc_pred;
                                        let mut k = 1;
                                        while k < 64 {
                                            let rs = br.decode(&ac_t[ta])?;
                                            let (r, s) = ((rs >> 4) as usize, (rs & 15) as u32);
                                            if s == 0 {
                                                if r == 15 {
                                                    k += 16;
                                                    continue;
                                                }
                                                break;
                                            }
                                            k += r;
                                            if k > 63 {
                                                break;
                                            }
                                            blk[ZIGZAG[k]] = extend(br.bits(s), s);
                                            k += 1;
                                        }
                                    } else if ss == 0 {
                                        // DC scan
                                        if ah == 0 {
                                            let s = dc_size(br.decode(&dc_t[td])?)?;
                                            let diff = extend(br.bits(s), s);
                                            c.dc_pred = c.dc_pred.wrapping_add(diff);
                                            blk[0] = c.dc_pred << al;
                                        } else if br.bit() == 1 {
                                            blk[0] |= 1 << al;
                                        }
                                    } else if ah == 0 {
                                        // AC first
                                        if eobrun > 0 {
                                            eobrun -= 1;
                                            continue;
                                        }
                                        let mut k = ss;
                                        while k <= se {
                                            let rs = br.decode(&ac_t[ta])?;
                                            let (r, s) = ((rs >> 4) as u32, (rs & 15) as u32);
                                            if s == 0 {
                                                if r < 15 {
                                                    eobrun = (1 << r) - 1;
                                                    if r > 0 {
                                                        eobrun += br.bits(r);
                                                    }
                                                    break;
                                                }
                                                k += 16;
                                                continue;
                                            }
                                            k += r as usize;
                                            if k > 63 {
                                                break;
                                            }
                                            blk[ZIGZAG[k]] = extend(br.bits(s), s) * (1 << al);
                                            k += 1;
                                        }
                                    } else {
                                        // AC refinement
                                        let p1 = 1i32 << al;
                                        let m1 = -1i32 << al;
                                        let mut k = ss;
                                        if eobrun == 0 {
                                            while k <= se {
                                                let rs = br.decode(&ac_t[ta])?;
                                                let (mut r, s) = ((rs >> 4) as i32, (rs & 15) as u32);
                                                let mut val = 0;
                                                if s == 0 {
                                                    if r < 15 {
                                                        eobrun = (1 << r) as u32;
                                                        if r > 0 {
                                                            eobrun += br.bits(r as u32);
                                                        }
                                                        break;
                                                    }
                                                } else {
                                                    val = if br.bit() == 1 { p1 } else { m1 };
                                                }
                                                while k <= se {
                                                    let z = ZIGZAG[k];
                                                    if blk[z] != 0 {
                                                        if br.bit() == 1 && (blk[z] & p1) == 0 {
                                                            blk[z] += if blk[z] >= 0 { p1 } else { m1 };
                                                        }
                                                    } else {
                                                        if r == 0 {
                                                            if val != 0 {
                                                                blk[z] = val;
                                                            }
                                                            k += 1;
                                                            break;
                                                        }
                                                        r -= 1;
                                                    }
                                                    k += 1;
                                                }
                                            }
                                        }
                                        if eobrun > 0 {
                                            while k <= se {
                                                let z = ZIGZAG[k];
                                                if blk[z] != 0 && br.bit() == 1 && (blk[z] & p1) == 0 {
                                                    blk[z] += if blk[z] >= 0 { p1 } else { m1 };
                                                }
                                                k += 1;
                                            }
                                            eobrun -= 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                // continue after the entropy coded data
                p += len + br.pos;
                continue;
            }
            _ => {}
        }
        p += len;
    }
    if comps.is_empty() || w == 0 || h == 0 {
        return Err("jpeg: no image".into());
    }
    // dequantise + IDCT into component planes
    let mut cos_t = [[0f32; 8]; 8];
    for (x, row) in cos_t.iter_mut().enumerate() {
        for (u, v) in row.iter_mut().enumerate() {
            let cu = if u == 0 { std::f32::consts::FRAC_1_SQRT_2 } else { 1.0 };
            *v = cu * (((2 * x + 1) * u) as f32 * std::f32::consts::PI / 16.0).cos() * 0.5;
        }
    }
    let planes: Vec<(usize, usize, Vec<u8>)> = comps
        .iter()
        .map(|c| {
            let pw = c.bw * 8;
            let ph = c.bh * 8;
            let mut plane = vec![0u8; pw * ph];
            let q = &qt[c.tq];
            for by in 0..c.bh {
                for bx in 0..c.bw {
                    let blk = &c.coefs[by * c.bw + bx];
                    let mut f = [0f32; 64];
                    for i in 0..64 {
                        f[i] = blk[i].wrapping_mul(q[i] as i32) as f32;
                    }
                    // rows
                    let mut t = [0f32; 64];
                    for v in 0..8 {
                        for x in 0..8 {
                            let mut s = 0.0;
                            for u in 0..8 {
                                s += cos_t[x][u] * f[v * 8 + u];
                            }
                            t[v * 8 + x] = s;
                        }
                    }
                    for x in 0..8 {
                        for y in 0..8 {
                            let mut s = 0.0;
                            for v in 0..8 {
                                s += cos_t[y][v] * t[v * 8 + x];
                            }
                            plane[(by * 8 + y) * pw + bx * 8 + x] = (s + 128.0).round().clamp(0.0, 255.0) as u8;
                        }
                    }
                }
            }
            (pw, ph, plane)
        })
        .collect();
    let mut img = Image { width: w, height: h, deep: false, data: vec![[0u16; 3]; w * h] };
    // samples with "fancy" (bilinear, centred) upsampling of the chroma planes
    let sample = |ci: usize, x: usize, y: usize| -> f32 {
        let c = &comps[ci];
        let (pw, ph, plane) = &planes[ci];
        if c.h == hmax && c.v == vmax {
            return plane[y * pw + x] as f32;
        }
        let fx = (x as f32 + 0.5) * c.h as f32 / hmax as f32 - 0.5;
        let fy = (y as f32 + 0.5) * c.v as f32 / vmax as f32 - 0.5;
        let cw = (w * c.h).div_ceil(hmax).min(*pw);
        let chh = (h * c.v).div_ceil(vmax).min(*ph);
        let x0 = fx.floor().clamp(0.0, (cw - 1) as f32) as usize;
        let y0 = fy.floor().clamp(0.0, (chh - 1) as f32) as usize;
        let x1 = (x0 + 1).min(cw - 1);
        let y1 = (y0 + 1).min(chh - 1);
        let ax = (fx - x0 as f32).clamp(0.0, 1.0);
        let ay = (fy - y0 as f32).clamp(0.0, 1.0);
        let g = |xx: usize, yy: usize| plane[yy * pw + xx] as f32;
        (g(x0, y0) * (1.0 - ax) + g(x1, y0) * ax) * (1.0 - ay) + (g(x0, y1) * (1.0 - ax) + g(x1, y1) * ax) * ay
    };
    for y in 0..h {
        for x in 0..w {
            let px = if comps.len() == 1 {
                let v = sample(0, x, y) as u16;
                [v, v, v]
            } else {
                let (yy, cb, cr) = (sample(0, x, y), sample(1, x, y) - 128.0, sample(2, x, y) - 128.0);
                if adobe_rgb {
                    [yy as u16, (cb + 128.0) as u16, (cr + 128.0) as u16]
                } else {
                    let r = yy + 1.402 * cr;
                    let g = yy - 0.344136 * cb - 0.714136 * cr;
                    let bb = yy + 1.772 * cb;
                    [
                        r.round().clamp(0.0, 255.0) as u16,
                        g.round().clamp(0.0, 255.0) as u16,
                        bb.round().clamp(0.0, 255.0) as u16,
                    ]
                }
            };
            img.data[y * w + x] = px;
        }
    }
    Ok(img)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_round_trip_with_own_encoder() {
        let (w, h) = (37, 23);
        let rgb: Vec<u8> = (0..w * h * 3).map(|i| (i * 7 % 251) as u8).collect();
        let png = crate::png::encode_rgb(w, h, &rgb);
        let img = decode(&png).unwrap();
        assert_eq!((img.width, img.height), (w, h));
        for i in 0..w * h {
            let p = img.data[i];
            assert_eq!([p[0] as u8, p[1] as u8, p[2] as u8], [rgb[i * 3], rgb[i * 3 + 1], rgb[i * 3 + 2]]);
        }
    }

    #[test]
    fn malformed_files_are_errors() {
        // PNG with a short IHDR
        let mut png = vec![0x89, b'P', b'N', b'G', 13, 10, 26, 10, 0, 0, 0, 2];
        png.extend_from_slice(b"IHDR\0\0\0\0\0\0");
        assert!(decode(&png).is_err());
        assert!(decode(b"BM\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0").is_err());
        let jpeg = |seg: &[u8]| {
            let mut j = vec![0xFF, 0xD8];
            j.extend_from_slice(seg);
            j.extend_from_slice(&[0xFF, 0xD9]);
            j
        };
        let sof = [0xFF, 0xC2, 0, 11, 8, 0, 8, 0, 8, 1, 1, 0x11, 0];
        for seg in [
            &[0xFF, 0xDB, 0, 4, 0, 1][..],                    // short DQT
            &[0xFF, 0xC4, 0, 6, 0, 1, 2, 3][..],              // short DHT
            &[0xFF, 0xC0, 0, 5, 8, 0, 8][..],                 // short SOF
            &[0xFF, 0xC0, 0, 8, 8, 0, 8, 0, 8, 3][..],        // SOF missing components
        ] {
            assert!(decode(&jpeg(seg)).is_err());
        }
        // scans: no components, short header, spectral end past 63
        for sos in [&[0xFF, 0xDA, 0, 6, 0, 0, 63, 0][..], &[0xFF, 0xDA, 0, 4, 1, 1][..], &[0xFF, 0xDA, 0, 8, 1, 1, 0, 1, 70, 0][..]] {
            let mut seg = sof.to_vec();
            seg.extend_from_slice(sos);
            assert!(decode(&jpeg(&seg)).is_err());
        }
        // a DC size symbol of 40 (one code of length 1)
        let mut seg = vec![0xFF, 0xC0, 0, 11, 8, 0, 8, 0, 8, 1, 1, 0x11, 0];
        seg.extend_from_slice(&[0xFF, 0xC4, 0, 20, 0x00, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 40]);
        seg.extend_from_slice(&[0xFF, 0xDA, 0, 8, 1, 1, 0, 0, 63, 0, 0, 0]);
        assert!(decode(&jpeg(&seg)).is_err());
    }

    #[test]
    fn pgm_samples_are_scaled_to_the_maximum() {
        let img = decode(b"P5 2 1 15\n\x00\x0F").unwrap();
        assert!(!img.deep);
        assert_eq!((img.data[0][0], img.data[1][0]), (0, 255));
        let img = decode(b"P5 2 1 1023\n\x01\xFF\x03\xFF").unwrap();
        assert!(img.deep);
        assert_eq!((img.data[0][0], img.data[1][0]), ((511u32 * 65535 / 1023) as u16, 65535));
        assert!(decode(b"P5 1 1 0\n\x00").is_err());
    }
}
