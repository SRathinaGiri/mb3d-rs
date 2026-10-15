//! DCT image denoising, the algorithm of GIMP's "Denoise DCT" (GEGL's
//! `denoise-dct`), written here from the paper: G. Yu, G. Sapiro, "DCT
//! image denoising: a simple and effective image denoising algorithm",
//! Image Processing On Line 1 (2011), https://doi.org/10.5201/ipol.2011.ys-dct
//! (no GEGL code is used).
//!
//! Every 8x8 patch of each colour channel (all of them, overlapping) is
//! transformed with the 2D DCT, its coefficients below 3 sigma are set to
//! zero (the mean stays), and the patch is transformed back; each pixel is
//! the average of the patches that cover it.  As GIMP, the channels R, G
//! and B are filtered each on its own (the paper's colour transform made
//! the pictures here slightly worse, and different from GIMP's).

/// The patch size.
const P: usize = 8;

/// The orthonormal DCT-II matrix: `c[k][n]`.
fn dct_matrix() -> [[f32; P]; P] {
    let mut c = [[0f32; P]; P];
    for (k, row) in c.iter_mut().enumerate() {
        let a = if k == 0 { (1.0 / P as f64).sqrt() } else { (2.0 / P as f64).sqrt() };
        for (n, v) in row.iter_mut().enumerate() {
            *v = (a * (std::f64::consts::PI * (2 * n + 1) as f64 * k as f64 / (2 * P) as f64).cos()) as f32;
        }
    }
    c
}

/// Denoises an RGB image (`rgb`, 3 values per pixel, 0..255) whose noise
/// has the standard deviation `sigma` (on the same scale); `threads`
/// bands are calculated at once.  Images smaller than a patch, and sigma
/// <= 0, come back unchanged.
pub fn dct_denoise(rgb: &[u8], w: usize, h: usize, sigma: f32, threads: usize) -> Vec<u8> {
    if sigma <= 0.0 || w < P || h < P || rgb.len() != w * h * 3 {
        return rgb.to_vec();
    }
    let mut ch = vec![vec![0f32; w * h]; 3];
    for (i, p) in rgb.chunks(3).enumerate() {
        for k in 0..3 {
            ch[k][i] = p[k] as f32;
        }
    }
    let c = dct_matrix();
    let thr = 3.0 * sigma;
    let threads = threads.clamp(1, h);
    let band = h.div_ceil(threads);
    // per band of output rows: the sum of the patch estimates and their count
    let out: Vec<Vec<[f32; 3]>> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                let (ch, c) = (&ch, &c);
                s.spawn(move || {
                    let (y0, y1) = (t * band, ((t + 1) * band).min(h));
                    let mut acc = vec![[0f32; 3]; (y1.saturating_sub(y0)) * w];
                    if y0 >= y1 {
                        return acc;
                    }
                    // the patches that cover rows y0..y1
                    let (py0, py1) = (y0.saturating_sub(P - 1), (y1 - 1).min(h - P));
                    let mut a = [[0f32; P]; P];
                    let mut tmp = [[0f32; P]; P];
                    for (k, chan) in ch.iter().enumerate() {
                        for py in py0..=py1 {
                            for px in 0..=w - P {
                                for (i, row) in a.iter_mut().enumerate() {
                                    row.copy_from_slice(&chan[(py + i) * w + px..(py + i) * w + px + P]);
                                }
                                // forward: c * a * c^T
                                for i in 0..P {
                                    for j in 0..P {
                                        tmp[i][j] = (0..P).map(|n| c[i][n] * a[n][j]).sum();
                                    }
                                }
                                for i in 0..P {
                                    for j in 0..P {
                                        a[i][j] = (0..P).map(|n| tmp[i][n] * c[j][n]).sum();
                                    }
                                }
                                // hard threshold (the mean, a[0][0], stays)
                                for (i, row) in a.iter_mut().enumerate() {
                                    for (j, v) in row.iter_mut().enumerate() {
                                        if (i, j) != (0, 0) && v.abs() < thr {
                                            *v = 0.0;
                                        }
                                    }
                                }
                                // inverse: c^T * a * c
                                for i in 0..P {
                                    for j in 0..P {
                                        tmp[i][j] = (0..P).map(|n| c[n][i] * a[n][j]).sum();
                                    }
                                }
                                for i in 0..P {
                                    let y = py + i;
                                    if y < y0 || y >= y1 {
                                        continue;
                                    }
                                    for j in 0..P {
                                        let v: f32 = (0..P).map(|n| tmp[i][n] * c[n][j]).sum();
                                        acc[(y - y0) * w + px + j][k] += v;
                                    }
                                }
                            }
                        }
                    }
                    acc
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    // the patches covering a pixel: limited near the borders
    let cover = |v: usize, n: usize| -> f32 { ((v.min(n - P) as i64 - v.saturating_sub(P - 1) as i64) + 1) as f32 };
    let mut res = vec![0u8; w * h * 3];
    let rows = out.iter().flat_map(|b| b.iter());
    for (i, s) in rows.enumerate() {
        let (x, y) = (i % w, i / w);
        let m = 1.0 / (cover(x, w) * cover(y, h));
        for k in 0..3 {
            res[i * 3 + k] = (s[k] * m).round().clamp(0.0, 255.0) as u8;
        }
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dct_is_orthonormal() {
        let c = dct_matrix();
        for i in 0..P {
            for j in 0..P {
                let d: f32 = (0..P).map(|n| c[i][n] * c[j][n]).sum();
                assert!((d - if i == j { 1.0 } else { 0.0 }).abs() < 1e-5);
            }
        }
    }

    /// A smooth picture stays, noise added to it is mostly removed.
    #[test]
    fn removes_noise_keeps_smooth_picture() {
        let (w, h) = (64, 48);
        let clean: Vec<u8> = (0..w * h).flat_map(|i| {
            let (x, y) = (i % w, i / w);
            [(40 + x * 2) as u8, (60 + y * 2) as u8, 128]
        }).collect();
        // a small sigma keeps the clean picture (within rounding)
        let same = dct_denoise(&clean, w, h, 0.5, 3);
        assert!(same.iter().zip(&clean).all(|(a, b)| (*a as i32 - *b as i32).abs() <= 1));
        // noise of sigma 10 (a fixed pseudo random sequence)
        let mut seed = 12345u32;
        let mut rnd = || {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed >> 8) as f32 / (1u32 << 24) as f32
        };
        let noisy: Vec<u8> = clean.iter().map(|&v| {
            let n = (rnd() + rnd() + rnd() + rnd() - 2.0) * 10.0 * 3f32.sqrt();
            (v as f32 + n).round().clamp(0.0, 255.0) as u8
        }).collect();
        let err = |a: &[u8]| (a.iter().zip(&clean).map(|(a, b)| (*a as f32 - *b as f32).powi(2)).sum::<f32>() / a.len() as f32).sqrt();
        let den = dct_denoise(&noisy, w, h, 10.0, 4);
        assert!(err(&den) < err(&noisy) * 0.5, "noisy {} denoised {}", err(&noisy), err(&den));
        // the bands do not change the result
        assert_eq!(den, dct_denoise(&noisy, w, h, 10.0, 1));
    }
}
