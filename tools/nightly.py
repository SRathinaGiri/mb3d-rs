#!/usr/bin/env python3
"""The nightly test run of one computer (.github/workflows/nightly.yml).

Renders the example parameter files of the MB3D repository, random MutaGen
variations of some of them, and one small render of every other kind
(Monte Carlo, tiles, the swarm, an animation, a mesh, voxels), each in a
process of its own with a time limit, so a crash or a hang is caught and
the rest goes on. The example pictures are compared with those of the
previous night (--ref). Everything goes to <out>/results.json, which
tools/nightly_issues.py turns into GitHub issues. Exits 0 unless the test
itself cannot run: failures are results, not errors.

    nightly.py --bin target/release/mb3d --mb3d ../mb3d --out out [--ref ref]
               [--seed N] [--only N]
"""

import argparse
import json
import os
import platform
import random
import shutil
import struct
import subprocess
import sys
import time
import zlib
from pathlib import Path

EXAMPLE_SCALE = "0.15"
EXAMPLE_SECS = 600
MUTANT_SECS = 300
KIND_SECS = 600
MUTAGEN_PARENTS = 6


def run(cmd, secs, cwd=None):
    """Runs cmd: (status, detail, seconds); status ok, error, crash or
    timeout; detail the end of its error output."""
    t0 = time.time()
    try:
        p = subprocess.run(cmd, capture_output=True, text=True, errors="replace", timeout=secs, cwd=cwd)
    except subprocess.TimeoutExpired:
        return "timeout", f"no result after {secs} s", secs
    dt = time.time() - t0
    err = (p.stderr or "").strip()
    tail = "\n".join(err.splitlines()[-25:])
    if "panicked at" in err or p.returncode < 0 or p.returncode == 101 or p.returncode > 128:
        return "crash", tail or f"exit code {p.returncode}", dt
    if p.returncode != 0:
        return "error", tail or f"exit code {p.returncode}", dt
    return "ok", "", dt


# ---------------------------------------------------------------------------
# PNG pictures (as the program writes them: 8 or 16 bit, grey, RGB or RGBA)


def png_pixels(path):
    """(width, height, channels, rows of samples) of a PNG, or None."""
    try:
        d = Path(path).read_bytes()
    except OSError:
        return None
    if d[:8] != b"\x89PNG\r\n\x1a\n":
        return None
    pos, idat, ihdr = 8, b"", None
    while pos + 8 <= len(d):
        n, kind = struct.unpack(">I4s", d[pos : pos + 8])
        body = d[pos + 8 : pos + 8 + n]
        if kind == b"IHDR":
            ihdr = struct.unpack(">IIBBBBB", body)
        elif kind == b"IDAT":
            idat += body
        pos += 12 + n
    # (not interlaced, no palette: as the program writes them)
    if not ihdr or ihdr[6] != 0 or ihdr[3] not in (0, 2, 4, 6) or ihdr[2] not in (8, 16):
        return None
    w, h, depth, ctype = ihdr[0], ihdr[1], ihdr[2], ihdr[3]
    ch = {0: 1, 2: 3, 4: 2, 6: 4}[ctype]
    bpp = max(1, ch * depth // 8)
    stride = (w * ch * depth + 7) // 8
    raw = zlib.decompress(idat)
    rows, prev = [], bytearray(stride)
    for y in range(h):
        f = raw[y * (stride + 1)]
        line = bytearray(raw[y * (stride + 1) + 1 : (y + 1) * (stride + 1)])
        for i in range(stride):
            a = line[i - bpp] if i >= bpp else 0
            b = prev[i]
            c = prev[i - bpp] if i >= bpp else 0
            if f == 1:
                line[i] = (line[i] + a) & 255
            elif f == 2:
                line[i] = (line[i] + b) & 255
            elif f == 3:
                line[i] = (line[i] + (a + b) // 2) & 255
            elif f == 4:
                pa, pb, pc = abs(b - c), abs(a - c), abs(a + b - 2 * c)
                line[i] = (line[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        rows.append(bytes(line))
        prev = line
    return w, h, ch, depth, rows


def compare(a, b):
    """None if the pictures are the same, else a description."""
    if Path(a).read_bytes() == Path(b).read_bytes():
        return None
    pa, pb = png_pixels(a), png_pixels(b)
    if not pa or not pb:
        return {"pixels": -1, "max": -1, "note": "not comparable"}
    if pa[:4] != pb[:4]:
        return {"pixels": -1, "max": -1, "note": f"size {pa[0]}x{pa[1]} was {pb[0]}x{pb[1]}"}
    w, h, ch, depth, ra = pa
    rb = pb[4]
    step = ch * depth // 8
    changed, most = 0, 0
    for la, lb in zip(ra, rb):
        if la == lb:
            continue
        for x in range(w):
            sa, sb = la[x * step : (x + 1) * step], lb[x * step : (x + 1) * step]
            if sa != sb:
                changed += 1
                most = max(most, max(abs(u - v) for u, v in zip(sa, sb)))
    return {"pixels": changed, "of": w * h, "max": most}


# ---------------------------------------------------------------------------


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bin", required=True)
    ap.add_argument("--mb3d", required=True, help="the MB3D repository (M3Parameter, M3Formulas, M3Maps)")
    ap.add_argument("--out", required=True)
    ap.add_argument("--ref", help="the pictures of the previous night")
    ap.add_argument("--seed", type=int, default=int(time.strftime("%Y%m%d")))
    ap.add_argument("--only", type=int, default=0, help="only the first N example files (a quick try)")
    a = ap.parse_args()

    exe = str(Path(a.bin).resolve())
    repo = Path(a.mb3d).resolve()
    out = Path(a.out).resolve()
    if out.exists():
        shutil.rmtree(out)
    pics = out / "pictures"
    pics.mkdir(parents=True)
    data = ["--formulas", str(repo / "M3Formulas"), "--maps", str(repo / "M3Maps")]
    # (unfinished renders of the program's own, kept with the run)
    os.environ["MB3D_RESUME_DIR"] = str(out / "resume")
    results = []

    def note(test, item, cmd, r, extra=None):
        status, detail, dt = r
        # (the command as it is run again in a checkout: mb3d/ the MB3D
        # repository, out/ the output)
        short = ["mb3d" if c == exe else c.replace(str(repo), "mb3d").replace(str(out), "out") for c in cmd]
        e = {"test": test, "item": item, "status": status, "detail": detail, "seconds": round(dt, 1), "command": " ".join(f'"{c}"' if " " in c else c for c in short)}
        if extra:
            e.update(extra)
        results.append(e)
        print(f"{status:8} {dt:7.1f} s  {test}: {item}", flush=True)

    examples = sorted((repo / "M3Parameter").glob("*.m3p"))
    if a.only:
        examples = examples[: a.only]

    # the example files, small
    for f in examples:
        cmd = [exe, "batch", str(f), "--scale", EXAMPLE_SCALE, "-o", str(pics), "-q"] + data
        note("example", f.name, cmd, run(cmd, EXAMPLE_SECS))

    # random variations (the seed is in the results: the same command makes them again)
    rnd = random.Random(a.seed)
    for f in rnd.sample(examples, min(MUTAGEN_PARENTS, len(examples))):
        d = out / "mutagen" / f.stem
        cmd = [exe, "mutagen", str(f), "-o", str(d), "--seed", str(a.seed), "--size", "96", "-q"] + data
        r = run(cmd, MUTANT_SECS * 3)
        note("mutagen", f.name, cmd, r, {"seed": a.seed})
        if r[0] != "ok":
            continue
        for child in sorted(d.glob("*.m3p")):
            cmd = [exe, "batch", str(child), "--scale", EXAMPLE_SCALE, "-o", str(d / "render"), "-q"] + data
            r = run(cmd, MUTANT_SECS)
            # (a mutation may well be unusable: only crashes and hangs count)
            if r[0] == "error":
                r = ("ok", "", r[2])
            note("mutant", f"{f.name} seed {a.seed} member {child.stem}", cmd, r, {"seed": a.seed})

    # one render of every other kind
    base = repo / "M3Parameter" / "Mandelbulb.m3p"
    if not base.exists() and examples:
        base = examples[0]
    k = out / "kinds"
    k.mkdir()
    small = ["-W", "240", "-H", "180"]
    kinds = [
        ("picture", [exe, "render", str(base), *small, "-o", str(k / "picture.png")]),
        ("tiles", [exe, "render", str(base), *small, "--tiles", "2x2", "-o", str(k / "tiles.png")]),
        ("swarm tiles", [exe, "render", str(base), *small, "--tiles", "2x2", "--swarm", str(k / "swarm"), "-o", str(k / "swarm.png")]),
        ("monte carlo", [exe, "montecarlo", str(base), "-s", "width=160", "-s", "height=120", "--passes", "2", "-o", str(k / "mc.png")]),
        ("animation", [exe, "animate", str(base), str(base), "--frames", "2", *small, "-o", str(k / "anim")]),
        ("mesh", [exe, "mesh", str(base), "--resolution", "48", "-o", str(k / "mesh.ply")]),
        ("voxels", [exe, "voxel", str(base), "--slices", "16", "-o", str(k / "voxels")]),
    ]
    for name, cmd in kinds:
        note("kind", name, cmd + ["-q"] + data, run(cmd + ["-q"] + data, KIND_SECS))

    # the example pictures against the previous night's
    changes = []
    if a.ref and Path(a.ref).is_dir():
        for p in sorted(pics.glob("*.png")):
            old = Path(a.ref) / p.name
            if old.exists():
                c = compare(p, old)
                if c:
                    changes.append({"item": p.name, **c})

    info = {
        "os": os.environ.get("NIGHTLY_OS") or platform.system().lower(),
        "sha": os.environ.get("GITHUB_SHA", ""),
        "seed": a.seed,
        "results": results,
        "changes": changes,
        "compared": bool(a.ref and Path(a.ref).is_dir()),
    }
    (out / "results.json").write_text(json.dumps(info, indent=1))
    bad = [r for r in results if r["status"] != "ok"]
    print(f"\n{len(results)} runs, {len(bad)} failed, {len(changes)} pictures changed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
