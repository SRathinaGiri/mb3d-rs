#!/usr/bin/env python3
"""Timing and picture sweeps over the example scenes that are not skewed.

Every sweep so far that was run by hand had numbers that turned out wrong
and had to be run again: the first render of a new shader on the card took
~30 s of compiling, a build ran during a sweep, the shell merged two
variants' arguments into one, a hole check looked for the wrong background,
a "processor" run went to the card.  This driver rules those out:

- a frozen copy of the program (its sha256 and the commit in the header);
- a quiet machine: before every run it waits while a build (cargo, rustc)
  runs or the processor is busy (another render), and logs the processor's
  idle share;
- two rounds, the second in reverse order of the variants; the time is the
  smaller one (a shader compile or a drift falls out), a pair more than 15 %
  apart is run a third time and the closest two count; the pictures are
  round 2's (in round 1 the processor fills in part of the first picture
  while the card's shaders compile);
- the variants as argument lists (JSON), never through a shell;
- every run checked: the device it ran on ("calculated on" in the output:
  the card with --card, never the card for "cpu"), the strings a variant
  expects (e.g. "cone march: on"), no error; a variant marked "same" must
  give the plain render's picture byte for byte on the processor, on the
  card within the card's own run-to-run variation (two identical card
  renders differ in ~0.001 % of the pixels by a few levels);
- the noise of the plain render itself: the scene's random first step
  flipped ("noise"; the scene's own setting is the plain render already); a
  variant changes a scene only where it differs from the plain render by
  more than that (pictures: pixels more than 8 of 255 apart;
  holes: surface in the plain render and background in the variant, the
  depth picture's background being 0).

Use:
    tools/bench.py run SWEEP.json OUT_DIR      # resumes an interrupted run
    tools/bench.py report OUT_DIR

SWEEP.json:
    {
      "device": "cpu",                  # cpu | card | auto: mb3d --cpu, --card, neither
      "megapixels": 0.2,                # each scene scaled to about this size
      "scenes": "~/mb3d-rs/M3Parameter/*.m3p",
      "formulas": "~/mb3d-rs/M3Formulas",
      "binary": "target/release/mb3d",  # copied into OUT_DIR
      "exclude": ["Fishbowl"],          # name prefixes left out
      "variants": {
        "cone": {"args": ["-s", "experimental=on", "-s", "cone_march=on"],
                 "expect": ["cone march: on"]},
        "cone_env": {"args": [...], "env": {"NAME": "value"}},
        "off": {"args": ["-s", "experimental=off"], "same": true}
      }
    }
The plain render ("base") and its noise ("noise") are always added.  The
report compares every variant with "base".  The settings are checked
before anything runs: unknown keys, a variant that repeats the plain
render or another variant, and arguments or variables the driver sets
itself (--card, --scale, -o, --depth, MB3D_GPU, ...) are refused.
"""
import glob
import hashlib
import json
import os
import re
import shutil
import statistics
import subprocess
import sys
import time

ROUNDS_APART = 0.15  # a pair of times further apart than this is run again
PIXEL_STEP = 8  # a pixel differs when a channel is more than this apart
SAME_CARD = 0.01  # % of pixels a "same" variant may differ in on the card
QUIET_IDLE = 0.85  # the processor's idle share a run waits for
# builds, by name; any other busy program shows in the idle share (an open,
# idle Mandelbulb3D window does not hold a sweep up)
BUSY = re.compile(r"^(cargo|rustc|cc1|cc1plus|ld|ld\.lld|lld|clang|clang-\d+)$")


def log(msg):
    print(msg, flush=True)


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for b in iter(lambda: f.read(1 << 20), b""):
            h.update(b)
    return h.hexdigest()


def idle_share(seconds=0.5):
    """The processor's idle share over a short time (/proc/stat)."""
    def read():
        v = [int(x) for x in open("/proc/stat").readline().split()[1:]]
        return v[3] + v[4], sum(v)
    i0, t0 = read()
    time.sleep(seconds)
    i1, t1 = read()
    return (i1 - i0) / max(1, t1 - t0)


def busy_processes():
    out = []
    for pid in os.listdir("/proc"):
        if not pid.isdigit() or int(pid) == os.getpid():
            continue
        try:
            name = open(f"/proc/{pid}/comm").read().strip()
        except OSError:
            continue
        if BUSY.match(name):
            out.append(name)
    return out


def wait_quiet():
    """Waits until no build or other render runs and the processor is idle;
    returns the idle share it measured."""
    waited = 0.0
    while True:
        busy = busy_processes()
        idle = idle_share()
        if not busy and idle >= QUIET_IDLE:
            if waited:
                log(f"    (waited {waited:.0f} s for a quiet machine)")
            return idle
        if waited == 0 or int(waited) % 60 < 5:
            log(f"    waiting for a quiet machine: {', '.join(sorted(set(busy))) or 'busy'}, idle {idle:.0%}")
        time.sleep(5)
        waited += 5.5


def scene_info(binary, scene, formulas, tmp):
    """The scene's size and its random first step, without rendering
    (--save-scene prints the size and writes the scene as text)."""
    r = subprocess.run([binary, scene, "--formulas", formulas, "--save-scene", tmp], capture_output=True, text=True, timeout=120)
    m = re.search(r", (\d+)x(\d+)\)", r.stdout + r.stderr)
    if not m or r.returncode != 0:
        return None
    first = re.search(r"^first_step_random = (\w+)", open(tmp).read(), re.M)
    return int(m.group(1)), int(m.group(2)), (first.group(1) == "true") if first else False


CONFIG_KEYS = {"device", "megapixels", "scenes", "formulas", "binary", "exclude", "variants"}
VARIANT_KEYS = {"args", "env", "expect", "same"}
# set by the driver: a variant must not repeat or contradict them
OWN_ARGS = {"--card", "--cpu", "--scale", "-o", "--output", "--depth", "--fresh", "--no-resume", "--formulas", "--save-scene", "--render"}
OWN_ENV = {"MB3D_GPU", "MB3D_STAGE_TIMES"}


def validate(cfg):
    """Refuses settings that repeat each other or the driver's own."""
    errs = [f"unknown setting {k!r} (known: {', '.join(sorted(CONFIG_KEYS))})" for k in cfg if k not in CONFIG_KEYS]
    if cfg.get("device") not in ("cpu", "card", "auto"):
        errs.append(f"device must be cpu, card or auto, not {cfg.get('device')!r}")
    if not isinstance(cfg.get("megapixels"), (int, float)) or not 0 < cfg["megapixels"] <= 50:
        errs.append("megapixels must be a number above 0 (at most 50)")
    if not cfg.get("variants"):
        errs.append("no variants")
    seen = {}
    for name, spec in cfg.get("variants", {}).items():
        if name in ("base", "noise"):
            errs.append(f"variant name {name!r} is the driver's own")
        errs += [f"variant {name}: unknown key {k!r} (known: {', '.join(sorted(VARIANT_KEYS))})" for k in spec if k not in VARIANT_KEYS]
        args, env = spec.get("args", []), spec.get("env", {})
        if not isinstance(args, list) or not all(isinstance(a, str) for a in args):
            errs.append(f"variant {name}: args must be a list of strings")
            continue
        errs += [f"variant {name}: {a} is set by the driver (device, megapixels)" for a in args if a.split("=")[0] in OWN_ARGS]
        errs += [f"variant {name}: {k} is set by the driver (device)" for k in env if k in OWN_ENV]
        if "-s" in args[-1:]:
            errs.append(f"variant {name}: -s without a value")
        keys = [args[i + 1].split("=")[0] for i, a in enumerate(args[:-1]) if a in ("-s", "--set")]
        errs += [f"variant {name}: scene key {k!r} set twice" for k in set(keys) if keys.count(k) > 1]
        if "first_step_random" in keys:
            errs.append(f"variant {name}: first_step_random is the noise variant's")
        if not args and not env:
            errs.append(f"variant {name}: no arguments or variables, the same as base")
        sig = (tuple(args), tuple(sorted(env.items())))
        if sig in seen:
            errs.append(f"variant {name}: the same arguments and variables as {seen[sig]}")
        seen[sig] = name
    if errs:
        sys.exit("the sweep's settings:\n  " + "\n  ".join(errs))


def variants_of(cfg):
    v = {"base": {"args": []}, "noise": {"args": []}}  # noise: per scene
    v.update(cfg["variants"])
    return v


def check(cfg, spec, out):
    """What is wrong with a run's output, or None."""
    m = re.search(r"calculated on: ([^\n]*)", out)
    on = m.group(1) if m else "CPU"
    if cfg["device"] == "card" and not on.startswith("GPU"):
        return f"meant for the card, calculated on: {on}"
    if cfg["device"] == "cpu" and on.startswith("GPU"):
        return f"meant for the processor, calculated on: {on}"
    for e in spec.get("expect", []):
        if e not in out:
            return f"not in use: {e!r} missing in the output"
    if not re.search(r"stage times: march [0-9.]+", out):
        return "no march time in the output"
    return None


def run(cfg_path, out_dir):
    cfg = json.load(open(cfg_path))
    cfg.setdefault("device", "cpu")
    cfg.setdefault("megapixels", 0.2)
    cfg.setdefault("scenes", "~/mb3d-rs/M3Parameter/*.m3p")
    cfg.setdefault("formulas", "~/mb3d-rs/M3Formulas")
    cfg.setdefault("binary", "target/release/mb3d")
    cfg.setdefault("exclude", [])
    validate(cfg)
    os.makedirs(f"{out_dir}/img", exist_ok=True)
    variants = variants_of(cfg)
    # the frozen program: copied once, the same one on a resume
    binary = os.path.join(out_dir, "mb3d")
    header = os.path.join(out_dir, "sweep.json")
    if os.path.exists(header):
        h = json.load(open(header))
        if sha256(binary) != h["sha256"]:
            sys.exit("the frozen program in OUT_DIR changed; start a new OUT_DIR")
        if h["config"] != cfg:
            sys.exit("the sweep's settings differ from the ones OUT_DIR was started with; start a new OUT_DIR")
    else:
        shutil.copy2(os.path.expanduser(cfg["binary"]), binary)
        commit = subprocess.run(["git", "describe", "--always", "--dirty"], capture_output=True, text=True).stdout.strip()
        json.dump({"config": cfg, "sha256": sha256(binary), "commit": commit, "started": time.strftime("%Y-%m-%d %H:%M:%S")}, open(header, "w"), indent=1)
        log(f"frozen program {sha256(binary)[:12]} from {commit}")
    results = os.path.join(out_dir, "runs.jsonl")
    done = {}
    if os.path.exists(results):
        for line in open(results):
            r = json.loads(line)
            done[(r["scene"], r["variant"], r["round"])] = r
    formulas = os.path.expanduser(cfg["formulas"])
    scenes = sorted(glob.glob(os.path.expanduser(cfg["scenes"])))
    scenes = [s for s in scenes if not any(os.path.basename(s).startswith(x) for x in cfg["exclude"])]
    # the device by mb3d's options alone (--cpu, --card; neither: Auto):
    # no MB3D_ variable of the caller's environment comes along (MB3D_GPU=0
    # would make "auto" the processor)
    base_env = {k: v for k, v in os.environ.items() if not k.startswith("MB3D_")}
    base_env["MB3D_STAGE_TIMES"] = "1"
    for si, scene in enumerate(scenes):
        name = os.path.splitext(os.path.basename(scene))[0]
        info = scene_info(binary, scene, formulas, os.path.join(out_dir, "scene.m3s"))
        if not info:
            log(f"[{si + 1}/{len(scenes)}] {name}: cannot be read, left out")
            continue
        w, h, first = info
        scale = min(1.0, (cfg["megapixels"] * 1e6 / (w * h)) ** 0.5)
        log(f"[{si + 1}/{len(scenes)}] {name} ({w}x{h}, scale {scale:.3f}, random first step {'on' if first else 'off'})")
        order = list(variants)

        def one(v, rnd):
            key = (name, v, rnd)
            if key in done:
                return done[key]
            spec = variants[v]
            if v == "noise":
                spec = {"args": ["-s", f"first_step_random={'false' if first else 'true'}"]}
            cmd = [binary, scene, "--formulas", formulas, "--fresh", "--no-resume", "--scale", f"{scale:.6f}"]
            if cfg["device"] != "auto":
                cmd.append("--" + cfg["device"])
            cmd += spec.get("args", [])
            img = f"{out_dir}/img/{v}-{name}"
            # the pictures from round 2: in round 1 the first variant of a
            # scene runs while the card's shaders still compile, and the
            # processor fills in part of its picture meanwhile
            cmd += ["-o", f"{img}.png", "--depth", f"{img}-z.png"] if rnd == 2 else ["-o", f"{out_dir}/scratch.png"]
            env = dict(base_env, **spec.get("env", {}))
            idle = wait_quiet()
            t0 = time.time()
            try:
                p = subprocess.run(cmd, env=env, capture_output=True, text=True, timeout=3600)
                out, code = p.stdout + p.stderr, p.returncode
            except subprocess.TimeoutExpired:
                out, code = "", "timeout after 3600 s"
            wall = time.time() - t0
            m = re.search(r"stage times: march ([0-9.]+)", out)
            bad = check(cfg, spec, out) if code == 0 else f"exit code {code}"
            r = {"scene": name, "variant": v, "round": rnd, "march": float(m.group(1)) if m else None, "wall": round(wall, 3), "idle_before": round(idle, 3), "error": bad, "cmd": cmd, "env": spec.get("env", {})}
            with open(results, "a") as f:
                f.write(json.dumps(r) + "\n")
            done[key] = r
            log(f"    {v:>12s} round {rnd}: march {r['march']}" + (f"  ** {bad}" if bad else ""))
            return r

        for v in order:
            one(v, 1)
        for v in reversed(order):
            one(v, 2)
        # a pair too far apart: a third run
        for v in order:
            r1, r2 = done[(name, v, 1)], done[(name, v, 2)]
            a, b = r1["march"], r2["march"]
            if a and b and not r1["error"] and not r2["error"] and abs(a - b) / min(a, b) > ROUNDS_APART:
                one(v, 3)
    log("FINISHED")


def report(out_dir):
    import numpy as np
    from PIL import Image

    h = json.load(open(f"{out_dir}/sweep.json"))
    cfg = h["config"]
    runs = {}
    errors = []
    for line in open(f"{out_dir}/runs.jsonl"):
        r = json.loads(line)
        runs.setdefault((r["scene"], r["variant"]), []).append(r)
        if r["error"]:
            errors.append((r["scene"], r["variant"], re.sub(r"calculated on: (GPU|CPU).*", r"calculated on: \1", r["error"])))
    scenes = sorted({s for s, _ in runs})

    def best(s, v):
        rs = [r for r in runs.get((s, v), []) if r["march"] and not r["error"]]
        return min(r["march"] for r in rs) if len(rs) >= 2 else None

    def spread(s, v):
        """How far apart the closest two runs are."""
        rs = sorted(r["march"] for r in runs.get((s, v), []) if r["march"] and not r["error"])
        return min((b - a) / a for a, b in zip(rs, rs[1:])) if len(rs) >= 2 else None

    cache = {}

    def load(s, v):
        if (s, v) not in cache:
            try:
                a = np.asarray(Image.open(f"{out_dir}/img/{v}-{s}.png").convert("RGB")).astype(int)
                z = np.asarray(Image.open(f"{out_dir}/img/{v}-{s}-z.png"))
                cache[(s, v)] = (a, z[..., 0] if z.ndim == 3 else z)
            except OSError:
                cache[(s, v)] = None
        return cache[(s, v)]

    def diff(s, v):
        a, b = load(s, "base"), load(s, v)
        if a is None or b is None or a[0].shape != b[0].shape:
            return None
        return ((np.abs(a[0] - b[0]).max(axis=2) > PIXEL_STEP).mean() * 100, ((a[1] > 0) & (b[1] == 0)).mean() * 100)

    def noise(s):
        return diff(s, "noise")

    def same(s, v):
        """The plain render's picture: byte for byte on the processor; on
        the card no pixel more than PIXEL_STEP apart and at most SAME_CARD
        of them different at all (the card's own variation)."""
        try:
            if open(f"{out_dir}/img/{v}-{s}.png", "rb").read() == open(f"{out_dir}/img/base-{s}.png", "rb").read():
                return True
        except OSError:
            return False
        if cfg["device"] == "cpu":
            return False
        a, b = load(s, "base"), load(s, v)
        if a is None or b is None or a[0].shape != b[0].shape:
            return False
        d = np.abs(a[0] - b[0]).max(axis=2)
        return (d > PIXEL_STEP).sum() == 0 and (d > 0).mean() * 100 <= SAME_CARD

    print(f"program {h['sha256'][:12]} ({h['commit']}), {cfg['device']}, {cfg['megapixels']} megapixels, started {h['started']}")
    print(f"times: the smaller of two rounds (the second in reverse order), a third run where they were more than {ROUNDS_APART:.0%} apart")
    for v, spec in cfg["variants"].items():
        ok = [s for s in scenes if best(s, v) and best(s, "base")]
        if not ok:
            print(f"\n{v}: no complete scenes")
            continue
        r = [best(s, v) / best(s, "base") - 1 for s in ok]
        tb = sum(best(s, "base") for s in ok)
        tv = sum(best(s, v) for s in ok)
        print(f"\n{v}: {len(ok)} scenes, march total {tv / tb * 100 - 100:+.1f} %, median {statistics.median(r) * 100:+.1f} %, best {min(r) * 100:+.0f} %, worst {max(r) * 100:+.0f} %, {sum(x < 0 for x in r)} faster")
        changed = []
        for s in ok:
            d, n = diff(s, v), noise(s)
            if spec.get("same"):
                if not same(s, v):
                    changed.append(f"{s}: not the same picture as base")
                continue
            if d and n and (d[0] > n[0] * 1.5 + 0.2 or d[1] > n[1] * 1.5 + 0.02):
                changed.append(f"{s}: pixels {d[0]:.2f} % (noise {n[0]:.2f} %), holes {d[1]:.3f} % (noise {n[1]:.3f} %)")
        print(f"  beyond the noise: {len(changed)}" + "".join(f"\n    {c}" for c in changed))
        wide = [f"{s} {spread(s, v) * 100:.0f} %" for s in ok if (spread(s, v) or 0) > ROUNDS_APART]
        if wide:
            print(f"  rounds still apart after the third run: {', '.join(wide)}")
    if errors:
        # by variant and message: a scene where a switch does not apply
        # (expect missing) or a device mix-up shows once per variant
        groups = {}
        for scene, variant, msg in errors:
            groups.setdefault((variant, msg), set()).add(scene)
        print(f"\nruns left out ({len(errors)}):")
        for (variant, msg), sc in sorted(groups.items()):
            print(f"  {variant}: {msg} — {len(sc)} scene(s): {', '.join(sorted(sc)[:6])}{' ...' if len(sc) > 6 else ''}")


if __name__ == "__main__":
    if len(sys.argv) == 4 and sys.argv[1] == "run":
        run(sys.argv[2], sys.argv[3])
    elif len(sys.argv) == 3 and sys.argv[1] == "report":
        report(sys.argv[2])
    else:
        sys.exit(__doc__)
