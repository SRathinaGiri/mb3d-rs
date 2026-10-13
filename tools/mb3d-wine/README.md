# MB3D under Wine: render a parameter file and save its G-buffer

Developer tools for comparing mb3d-rs with the original Mandelbulb3D 1.99
(run under Wine) on Linux with X11 / XWayland. Used for the 180430,
DeviantArt and USB scene comparisons (see `examples/gbcompare.rs`).

- `xw.py` — X11 helper (ctypes, no dependencies): `list` windows,
  `grab TITLE out.png`, `click X Y`, `where X Y`, `type TEXT [enter]`.
- `mbrun.sh N` — starts MB3D (`$MB3D_WINE_DIR`, default
  `~/Mandelbulb3Dv199sr37`) with `$MB3D_WINE_WORK/da/m3p/sN.m3p`, clicks
  Calculate 3D and waits until the button is back (compared with
  `btn_ready.png`), then grabs the window to `da/mb_sN.png`.
- `mbsave.sh N` — `mbrun.sh N`, then Save tab, "m3i" button, types the
  path and saves MB3D's image + G-buffer as `da/mb_sN.m3i`; closes MB3D.

`$MB3D_WINE_WORK` defaults to the current folder. The click coordinates
assume MB3D's main window at screen position (48, 169) and its save dialog
at (963, 568), where Wine placed them on the 1920x1080 test desktop: check
with `xw.py list` and adjust if yours differ. Write the `.m3p` files with
`examples/txt2m3p.rs` (scales an image, SET= changes keys). The image of a
640 px wide scene is the crop `640xH+7+102` of `mb_sN.png`.

Compare a G-buffer: `cargo run --release --example gbcompare -- MB3D_DIR
sN.m3p da/mb_sN.m3i OUTDIR` prints the field differences and paints MB3D's
G-buffer with our painter (`OUTDIR/theirs.png`).
