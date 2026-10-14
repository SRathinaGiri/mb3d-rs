//! Minimal dependency-free PNG writer (RGB8 and Gray16).
//!
//! Deflate is implemented with a hash-chain LZ77 matcher and the fixed
//! Huffman code table, which gives reasonable file sizes for rendered images
//! without pulling in external crates.

use std::io::Write;

fn crc32_table() -> [u32; 256] {
    let mut t = [0u32; 256];
    for (n, e) in t.iter_mut().enumerate() {
        let mut c = n as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB88320 ^ (c >> 1) } else { c >> 1 };
        }
        *e = c;
    }
    t
}

fn crc32(table: &[u32; 256], data: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c = table[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFF_FFFF
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for chunk in data.chunks(5552) {
        for &x in chunk {
            a += x as u32;
            b += a;
        }
        a %= 65521;
        b %= 65521;
    }
    (b << 16) | a
}

/// Bit writer for deflate (LSB first).
struct Bits {
    out: Vec<u8>,
    acc: u64,
    n: u32,
}

impl Bits {
    fn put(&mut self, v: u32, len: u32) {
        self.acc |= (v as u64) << self.n;
        self.n += len;
        while self.n >= 8 {
            self.out.push(self.acc as u8);
            self.acc >>= 8;
            self.n -= 8;
        }
    }
    /// Huffman codes are written MSB first.
    fn put_rev(&mut self, code: u32, len: u32) {
        let mut r = 0;
        for i in 0..len {
            r |= ((code >> i) & 1) << (len - 1 - i);
        }
        self.put(r, len);
    }
    fn finish(mut self) -> Vec<u8> {
        if self.n > 0 {
            self.out.push(self.acc as u8);
        }
        self.out
    }
}

fn put_lit(b: &mut Bits, lit: u32) {
    match lit {
        0..=143 => b.put_rev(0x30 + lit, 8),
        144..=255 => b.put_rev(0x190 + lit - 144, 9),
        256..=279 => b.put_rev(lit - 256, 7),
        _ => b.put_rev(0xC0 + lit - 280, 8),
    }
}

const LEN_BASE: [u32; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LEN_EXTRA: [u32; 29] =
    [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0];
const DIST_BASE: [u32; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u32; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

fn put_match(b: &mut Bits, len: u32, dist: u32) {
    let li = LEN_BASE.iter().rposition(|&x| x <= len).unwrap();
    put_lit(b, 257 + li as u32);
    b.put(len - LEN_BASE[li], LEN_EXTRA[li]);
    let di = DIST_BASE.iter().rposition(|&x| x <= dist).unwrap();
    b.put_rev(di as u32, 5);
    b.put(dist - DIST_BASE[di], DIST_EXTRA[di]);
}

/// Deflate with fixed Huffman codes and a hash-chain LZ77 matcher.
fn deflate_fixed(data: &[u8]) -> Vec<u8> {
    const WIN: usize = 32768;
    const HBITS: usize = 15;
    let mut head = vec![usize::MAX; 1 << HBITS];
    let mut prev = vec![usize::MAX; data.len()];
    let hash = |i: usize| -> usize {
        let v = (data[i] as u32) | (data[i + 1] as u32) << 8 | (data[i + 2] as u32) << 16;
        (v.wrapping_mul(0x9E3779B1) >> (32 - HBITS)) as usize
    };
    let mut b = Bits { out: Vec::with_capacity(data.len() / 2), acc: 0, n: 0 };
    b.put(1, 1); // final block
    b.put(1, 2); // fixed huffman
    let mut i = 0;
    while i < data.len() {
        let mut best_len = 0;
        let mut best_dist = 0;
        if i + 3 <= data.len() {
            let h = hash(i);
            let mut cand = head[h];
            let mut chain = 0;
            while cand != usize::MAX && i - cand <= WIN && chain < 32 {
                let max = (data.len() - i).min(258);
                let mut l = 0;
                while l < max && data[cand + l] == data[i + l] {
                    l += 1;
                }
                if l > best_len {
                    best_len = l;
                    best_dist = i - cand;
                    if l == max {
                        break;
                    }
                }
                cand = prev[cand];
                chain += 1;
            }
            prev[i] = head[h];
            head[h] = i;
        }
        if best_len >= 3 {
            put_match(&mut b, best_len as u32, best_dist as u32);
            for k in i + 1..i + best_len {
                if k + 3 <= data.len() {
                    let h = hash(k);
                    prev[k] = head[h];
                    head[h] = k;
                }
            }
            i += best_len;
        } else {
            put_lit(&mut b, data[i] as u32);
            i += 1;
        }
    }
    put_lit(&mut b, 256);
    b.finish()
}

fn zlib(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    out.extend(deflate_fixed(data));
    out.extend(adler32(data).to_be_bytes());
    out
}

fn chunk(out: &mut Vec<u8>, table: &[u32; 256], kind: &[u8; 4], data: &[u8]) {
    out.extend((data.len() as u32).to_be_bytes());
    let mut c = Vec::with_capacity(4 + data.len());
    c.extend_from_slice(kind);
    c.extend_from_slice(data);
    out.extend(&c);
    out.extend(crc32(table, &c).to_be_bytes());
}

/// Encode a PNG. `bytes_per_pixel` = 3 (RGB8) or 2 (Gray16 big endian).
fn encode(width: usize, height: usize, pixels: &[u8], color_type: u8, bit_depth: u8, bpp: usize) -> Vec<u8> {
    let table = crc32_table();
    let mut raw = Vec::with_capacity((width * bpp + 1) * height);
    // filter: Sub for each row (works well for smooth renders)
    for y in 0..height {
        let row = &pixels[y * width * bpp..(y + 1) * width * bpp];
        raw.push(1);
        for (i, &v) in row.iter().enumerate() {
            let left = if i >= bpp { row[i - bpp] } else { 0 };
            raw.push(v.wrapping_sub(left));
        }
    }
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::new();
    ihdr.extend((width as u32).to_be_bytes());
    ihdr.extend((height as u32).to_be_bytes());
    ihdr.extend([bit_depth, color_type, 0, 0, 0]);
    chunk(&mut out, &table, b"IHDR", &ihdr);
    chunk(&mut out, &table, b"IDAT", &zlib(&raw));
    chunk(&mut out, &table, b"IEND", &[]);
    out
}

/// PNG file contents of a 1 bit greyscale image (`Save1bitPNG`): `on`
/// pixels are white.
pub fn encode_gray1(width: usize, height: usize, on: &[bool]) -> Vec<u8> {
    let table = crc32_table();
    let rb = width.div_ceil(8);
    let mut raw = Vec::with_capacity((rb + 1) * height);
    for y in 0..height {
        raw.push(0);
        for b in 0..rb {
            let mut byte = 0u8;
            for k in 0..8 {
                let x = b * 8 + k;
                if x < width && on[y * width + x] {
                    byte |= 0x80 >> k;
                }
            }
            raw.push(byte);
        }
    }
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::new();
    ihdr.extend((width as u32).to_be_bytes());
    ihdr.extend((height as u32).to_be_bytes());
    ihdr.extend([1, 0, 0, 0, 0]);
    chunk(&mut out, &table, b"IHDR", &ihdr);
    chunk(&mut out, &table, b"IDAT", &zlib(&raw));
    chunk(&mut out, &table, b"IEND", &[]);
    out
}

/// PNG file contents of an 8 bit RGB image.
pub fn encode_rgb(width: usize, height: usize, rgb: &[u8]) -> Vec<u8> {
    encode(width, height, rgb, 2, 8, 3)
}

pub fn write_rgb(path: &str, width: usize, height: usize, rgb: &[u8]) -> std::io::Result<()> {
    let data = encode(width, height, rgb, 2, 8, 3);
    std::fs::File::create(path)?.write_all(&data)
}

/// `png` (from the encoders here) with a tEXt chunk `key` = `text` before
/// its IEND chunk; the text is stored as Latin-1 (other characters as '?').
pub fn with_text(mut png: Vec<u8>, key: &str, text: &str) -> Vec<u8> {
    let latin1 = |s: &str| s.chars().map(|c| if (c as u32) < 256 && c != '\0' { c as u8 } else { b'?' }).collect::<Vec<u8>>();
    let mut data = latin1(key);
    data.push(0);
    data.extend(latin1(text));
    let iend = png.split_off(png.len() - 12);
    chunk(&mut png, &crc32_table(), b"tEXt", &data);
    png.extend(iend);
    png
}

/// PNG file contents of a 16 bit RGB image.
pub fn encode_rgb16(width: usize, height: usize, rgb: &[u16]) -> Vec<u8> {
    let bytes: Vec<u8> = rgb.iter().flat_map(|v| v.to_be_bytes()).collect();
    encode(width, height, &bytes, 2, 16, 6)
}

/// PNG file contents of an 8 bit greyscale image.
pub fn encode_gray8(width: usize, height: usize, gray: &[u8]) -> Vec<u8> {
    encode(width, height, gray, 0, 8, 1)
}

/// PNG file contents of a 16 bit greyscale image.
pub fn encode_gray16(width: usize, height: usize, gray: &[u16]) -> Vec<u8> {
    let bytes: Vec<u8> = gray.iter().flat_map(|v| v.to_be_bytes()).collect();
    encode(width, height, &bytes, 0, 16, 2)
}

pub fn write_gray16(path: &str, width: usize, height: usize, gray: &[u16]) -> std::io::Result<()> {
    std::fs::File::create(path)?.write_all(&encode_gray16(width, height, gray))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb16_reads_back() {
        let px: Vec<u16> = vec![0, 1, 65535, 300, 40000, 12345];
        let img = crate::image::decode(&encode_rgb16(2, 1, &px)).unwrap();
        assert!(img.deep);
        assert_eq!(img.data.iter().flat_map(|p| [p[0], p[1], p[2]]).collect::<Vec<u16>>(), px);
    }

    #[test]
    fn crc_known_value() {
        let t = crc32_table();
        assert_eq!(crc32(&t, b"IEND"), 0xAE426082);
    }

    #[test]
    fn text_chunk_before_iend() {
        let p = with_text(encode_rgb(1, 1, &[1, 2, 3]), "Comment", "Mandelbulb3Dv18{a}");
        let i = p.windows(4).position(|w| w == b"tEXt").unwrap();
        assert_eq!(&p[i + 4..i + 12], b"Comment\0");
        assert_eq!(u32::from_be_bytes(p[i - 4..i].try_into().unwrap()), 26);
        assert_eq!(&p[p.len() - 8..p.len() - 4], b"IEND");
    }

    #[test]
    fn adler_known_value() {
        assert_eq!(adler32(b"Wikipedia"), 0x11E60398);
    }
}
