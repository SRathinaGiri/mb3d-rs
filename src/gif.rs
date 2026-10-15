//! Animated GIF output: the "AniGif" button of the animation preview
//! (MB3D's `GifAnimateBegin` / `GifAnimateAddImage`).  Each frame gets its
//! own 256 colour palette (median cut on the colours reduced to 5 bit per
//! channel), the pixels the nearest of its colours; the animation loops.

/// One frame: RGB, 3 bytes per pixel.
pub struct Frame<'a> {
    pub rgb: &'a [u8],
    pub w: usize,
    pub h: usize,
}

/// The animated GIF of `frames` (all of the size of the first one), each
/// shown `delay_cs` hundredths of a second.
pub fn encode_animation(frames: &[Frame], delay_cs: u16) -> Vec<u8> {
    let (w, h) = frames.first().map_or((1, 1), |f| (f.w, f.h));
    let mut o = Vec::new();
    o.extend_from_slice(b"GIF89a");
    o.extend_from_slice(&(w as u16).to_le_bytes());
    o.extend_from_slice(&(h as u16).to_le_bytes());
    // no global colour table
    o.extend_from_slice(&[0x00, 0, 0]);
    // loop for ever (NETSCAPE2.0)
    o.extend_from_slice(&[0x21, 0xFF, 11]);
    o.extend_from_slice(b"NETSCAPE2.0");
    o.extend_from_slice(&[3, 1, 0, 0, 0]);
    for f in frames {
        let (pal, idx) = quantize(f.rgb, f.w * f.h);
        // graphic control: the delay
        o.extend_from_slice(&[0x21, 0xF9, 4, 0x04]);
        o.extend_from_slice(&delay_cs.to_le_bytes());
        o.extend_from_slice(&[0, 0]);
        // image descriptor with a local colour table of 256 entries
        o.push(0x2C);
        o.extend_from_slice(&[0, 0, 0, 0]);
        o.extend_from_slice(&(f.w as u16).to_le_bytes());
        o.extend_from_slice(&(f.h as u16).to_le_bytes());
        o.push(0x80 | 7);
        for c in &pal {
            o.extend_from_slice(c);
        }
        o.extend(std::iter::repeat_n(0u8, (256 - pal.len()) * 3));
        o.push(8);
        for chunk in lzw(&idx).chunks(255) {
            o.push(chunk.len() as u8);
            o.extend_from_slice(chunk);
        }
        o.push(0);
    }
    o.push(0x3B);
    o
}

/// The 5 bit per channel key of a colour.
fn key(c: &[u8]) -> usize {
    ((c[0] as usize >> 3) << 10) | ((c[1] as usize >> 3) << 5) | (c[2] as usize >> 3)
}

/// Median cut: at most 256 colours, and each pixel's colour number.
fn quantize(rgb: &[u8], n: usize) -> (Vec<[u8; 3]>, Vec<u8>) {
    let mut count = vec![0u32; 32768];
    let mut sum = vec![[0u64; 3]; 32768];
    for c in rgb[..n * 3].chunks(3) {
        let k = key(c);
        count[k] += 1;
        for i in 0..3 {
            sum[k][i] += c[i] as u64;
        }
    }
    let used: Vec<usize> = (0..32768).filter(|&k| count[k] > 0).collect();
    let chan = |k: usize, i: usize| (k >> (10 - 5 * i)) & 31;
    // the boxes: lists of keys; the one with the widest channel range (of
    // those with more than one colour) is split at its pixel median
    let mut boxes: Vec<Vec<usize>> = vec![used];
    while boxes.len() < 256 {
        let mut best = None;
        for (b, keys) in boxes.iter().enumerate() {
            if keys.len() < 2 {
                continue;
            }
            for i in 0..3 {
                let (lo, hi) = keys.iter().fold((31, 0), |(lo, hi), &k| (lo.min(chan(k, i)), hi.max(chan(k, i))));
                let r = hi - lo;
                if best.is_none_or(|(_, _, br)| r > br) {
                    best = Some((b, i, r));
                }
            }
        }
        let Some((b, i, _)) = best else { break };
        let mut keys = std::mem::take(&mut boxes[b]);
        keys.sort_by_key(|&k| chan(k, i));
        let total: u64 = keys.iter().map(|&k| count[k] as u64).sum();
        let (mut acc, mut cut) = (0u64, 1);
        for (j, &k) in keys.iter().enumerate() {
            acc += count[k] as u64;
            if acc * 2 >= total {
                cut = (j + 1).clamp(1, keys.len() - 1);
                break;
            }
        }
        let rest = keys.split_off(cut);
        boxes[b] = keys;
        boxes.push(rest);
    }
    let pal: Vec<[u8; 3]> = boxes
        .iter()
        .filter(|b| !b.is_empty())
        .map(|keys| {
            let c: u64 = keys.iter().map(|&k| count[k] as u64).sum::<u64>().max(1);
            std::array::from_fn(|i| ((keys.iter().map(|&k| sum[k][i]).sum::<u64>() + c / 2) / c) as u8)
        })
        .collect();
    // each used key: the nearest palette colour
    let mut map = vec![0u8; 32768];
    for k in (0..32768).filter(|&k| count[k] > 0) {
        let c: [i32; 3] = std::array::from_fn(|i| (sum[k][i] / count[k] as u64) as i32);
        let near = pal
            .iter()
            .enumerate()
            .min_by_key(|(_, p)| (0..3).map(|i| (p[i] as i32 - c[i]).pow(2)).sum::<i32>())
            .map_or(0, |(j, _)| j);
        map[k] = near as u8;
    }
    let idx = rgb[..n * 3].chunks(3).map(|c| map[key(c)]).collect();
    (pal, idx)
}

/// The LZW codes (8 bit colours) of the image data, packed least
/// significant bit first.
fn lzw(idx: &[u8]) -> Vec<u8> {
    const CLEAR: u16 = 256;
    let mut out = Vec::new();
    let (mut acc, mut bits) = (0u32, 0u32);
    let mut put = |out: &mut Vec<u8>, code: u16, size: u32| {
        acc |= (code as u32) << bits;
        bits += size;
        while bits >= 8 {
            out.push(acc as u8);
            acc >>= 8;
            bits -= 8;
        }
    };
    let mut dict: std::collections::HashMap<u32, u16> = std::collections::HashMap::new();
    let mut size = 9u32;
    let mut max = CLEAR + 1;
    put(&mut out, CLEAR, size);
    let Some((&first, rest)) = idx.split_first() else {
        put(&mut out, CLEAR + 1, size);
        if bits > 0 {
            out.push(acc as u8);
        }
        return out;
    };
    let mut cur = first as u16;
    for &v in rest {
        let k = (cur as u32) << 8 | v as u32;
        if let Some(&c) = dict.get(&k) {
            cur = c;
            continue;
        }
        put(&mut out, cur, size);
        max += 1;
        dict.insert(k, max);
        if max as u32 >= 1 << size {
            size += 1;
        }
        if max == 4095 {
            put(&mut out, CLEAR, size);
            dict.clear();
            size = 9;
            max = CLEAR + 1;
        }
        cur = v as u16;
    }
    put(&mut out, cur, size);
    put(&mut out, CLEAR, size);
    put(&mut out, CLEAR + 1, 9);
    if bits > 0 {
        out.push(acc as u8);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A plain GIF LZW decoder (for the round trip).
    fn unlzw(data: &[u8]) -> Vec<u8> {
        let (mut pos, mut size) = (0usize, 9u32);
        let get = |pos: &mut usize, size: u32| -> u16 {
            let mut v = 0u32;
            for b in 0..size as usize {
                let bit = (data[(*pos + b) / 8] >> ((*pos + b) % 8)) & 1;
                v |= (bit as u32) << b;
            }
            *pos += size as usize;
            v as u16
        };
        let mut dict: Vec<Vec<u8>> = Vec::new();
        let reset = |d: &mut Vec<Vec<u8>>| {
            d.clear();
            d.extend((0..=255u8).map(|i| vec![i]));
            d.push(vec![]);
            d.push(vec![]);
        };
        reset(&mut dict);
        let mut out = Vec::new();
        let mut prev: Option<Vec<u8>> = None;
        loop {
            let c = get(&mut pos, size);
            if c == 256 {
                reset(&mut dict);
                size = 9;
                prev = None;
                continue;
            }
            if c == 257 {
                break;
            }
            let e = if (c as usize) < dict.len() {
                dict[c as usize].clone()
            } else {
                let p = prev.clone().unwrap();
                let mut e = p.clone();
                e.push(p[0]);
                e
            };
            out.extend_from_slice(&e);
            if let Some(p) = prev {
                let mut n = p;
                n.push(e[0]);
                dict.push(n);
                if dict.len() >= 1 << size && size < 12 {
                    size += 1;
                }
            }
            prev = Some(e);
        }
        out
    }

    #[test]
    fn lzw_round_trip() {
        let mut seed = 7u32;
        // long enough for several table resets, mixed runs and noise
        let data: Vec<u8> = (0..200_000)
            .map(|i| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                if i % 3000 < 1500 { (i / 700) as u8 } else { (seed >> 24) as u8 }
            })
            .collect();
        assert_eq!(unlzw(&lzw(&data)), data);
        assert_eq!(unlzw(&lzw(&[5])), vec![5]);
    }

    #[test]
    fn few_colours_stay_exact() {
        let cols = [[255u8, 0, 0], [0, 128, 255], [10, 200, 30]];
        let rgb: Vec<u8> = (0..300).flat_map(|i| cols[i % 3]).collect();
        let (pal, idx) = quantize(&rgb, 300);
        for (i, &k) in idx.iter().enumerate() {
            assert_eq!(pal[k as usize], cols[i % 3]);
        }
        let g = encode_animation(&[Frame { rgb: &rgb, w: 20, h: 15 }, Frame { rgb: &rgb, w: 20, h: 15 }], 10);
        assert!(g.starts_with(b"GIF89a") && g.ends_with(&[0x3B]));
    }
}
