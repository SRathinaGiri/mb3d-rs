#!/usr/bin/env python3
"""Watch projects built on mb3d-rs and report which of our code they use.

    python3 -I tools/fork_watch.py            # check now, write the report
    python3 -I tools/fork_watch.py --install  # check once a day (systemd user timer)

Watched: Rodrigo-Spacecraft/Mandelbulb3D-SE (a macOS app with our engine
copied into vendor/mb3d-rs) and every GitHub fork of thr32423/mb3d-rs.
Their code is only fetched and read, never built or run.

For a copied engine the report says which commit of ours it was taken from
(ENGINE_PROVENANCE.txt, else the closest commit by content), which of our
files are kept as they are, which are changed (and how much), which are new,
which of our files are left out, and which of our functions the app calls.
For a fork: the commits it has that we do not. Each run compares with the
last one and lists what is new (commits, snapshot, files); a desktop
notification says so when something changed.

Files: ~/.local/share/mb3d-fork-watch/ (clones, state.json, latest.md and a
dated report per run).
"""
import datetime
import difflib
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path

OURS = Path(__file__).resolve().parent.parent
HOME = Path(os.environ.get("XDG_DATA_HOME", Path.home() / ".local/share")) / "mb3d-fork-watch"
UPSTREAM = "thr32423/mb3d-rs"
COPIES = {"Rodrigo-Spacecraft/Mandelbulb3D-SE": "vendor/mb3d-rs"}
SKIP = {".git", "target"}


def run(*a, cwd=None, ok=False):
    r = subprocess.run(a, cwd=cwd, capture_output=True, text=True)
    if r.returncode and not ok:
        raise RuntimeError(f"{' '.join(a)}: {r.stderr.strip()}")
    return r.stdout


def forks():
    try:
        out = run("gh", "api", f"repos/{UPSTREAM}/forks", "--paginate", "-q", ".[].full_name")
        return [l for l in out.split() if l]
    except Exception:
        return []


def fetch(repo):
    d = HOME / "clones" / repo.replace("/", "__")
    url = f"https://github.com/{repo}.git"
    # hooks never run on clone/fetch; nothing in the clone is executed
    if not d.exists():
        d.parent.mkdir(parents=True, exist_ok=True)
        run("git", "-c", "core.hooksPath=/dev/null", "clone", "-q", url, str(d))
    else:
        run("git", "-c", "core.hooksPath=/dev/null", "fetch", "-q", "--tags", "--prune", "origin", cwd=d)
        branch = run("git", "rev-parse", "--abbrev-ref", "origin/HEAD", cwd=d).strip()
        run("git", "reset", "-q", "--hard", branch, cwd=d)
    return d


def blob(data):
    return hashlib.sha1(b"blob %d\0" % len(data) + data).hexdigest()


def tree_of(commit, prefix=""):
    """path -> blob id of our tree at commit"""
    out = run("git", "ls-tree", "-r", commit, cwd=OURS)
    t = {}
    for l in out.splitlines():
        meta, path = l.split("\t", 1)
        t[path] = meta.split()[2]
    return t


def files_of(root):
    t = {}
    for p in root.rglob("*"):
        if p.is_file() and not SKIP & set(p.relative_to(root).parts):
            t[str(p.relative_to(root))] = p
    return t


def snapshot_commit(app, copy):
    prov = app / "vendor" / "ENGINE_PROVENANCE.txt"
    if prov.exists():
        m = re.search(r"\b([0-9a-f]{40})\b", prov.read_text(errors="replace"))
        if m and run("git", "cat-file", "-t", m.group(1), cwd=OURS, ok=True).strip() == "commit":
            return m.group(1), "named in vendor/ENGINE_PROVENANCE.txt"
    # else the commit of ours with the most identical files
    theirs = {k: blob(v.read_bytes()) for k, v in files_of(copy).items()}
    best = (None, -1)
    for c in run("git", "rev-list", "--first-parent", "-n", "400", "HEAD", cwd=OURS).split():
        t = tree_of(c)
        n = sum(1 for k, b in theirs.items() if t.get(k) == b)
        if n > best[1]:
            best = (c, n)
    return best[0], f"closest by content ({best[1]} identical files)"


def describe(c):
    return run("git", "log", "-1", "--format=%h %ad %s", "--date=short", c, cwd=OURS).strip()[:110]


def version_at(c):
    m = re.search(r'^version = "([^"]+)"', run("git", "show", f"{c}:Cargo.toml", cwd=OURS), re.M)
    return m.group(1) if m else "?"


def changed_lines(c, path, data):
    old = run("git", "show", f"{c}:{path}", cwd=OURS, ok=True).splitlines()
    new = data.decode(errors="replace").splitlines()
    plus = minus = 0
    for l in difflib.unified_diff(old, new, lineterm="", n=0):
        if l.startswith("+") and not l.startswith("+++"):
            plus += 1
        elif l.startswith("-") and not l.startswith("---"):
            minus += 1
    return plus, minus


def later_version(path, b, snap):
    """the newer commit of ours whose file this is, if they took one"""
    for c in run("git", "log", "--format=%H", f"{snap}..HEAD", "--", path, cwd=OURS).split():
        if run("git", "rev-parse", f"{c}:{path}", cwd=OURS, ok=True).strip() == b:
            return c
    return None


def check_copy(repo, app, sub):
    copy = app / sub
    snap, how = snapshot_commit(app, copy)
    ours = tree_of(snap)
    head = tree_of("HEAD")
    kept, changed, newer, new = [], [], [], []
    for path, p in sorted(files_of(copy).items()):
        data = p.read_bytes()
        b = blob(data)
        if ours.get(path) == b:
            kept.append(path)
        elif path in ours:
            later = later_version(path, b, snap)
            if later:
                newer.append((path, later))
            else:
                changed.append((path, *changed_lines(snap, path, data)))
        else:
            later = later_version(path, b, snap) if path in head else None
            (newer.append((path, later)) if later else new.append((path, data.count(b"\n"))))
    present = set(files_of(copy))
    left_out = sorted(k for k in ours if k not in present)
    calls = {}
    for rs in app.rglob("*"):
        if rs.suffix in (".rs",) and sub not in str(rs.relative_to(app)) and "target" not in rs.parts:
            for m in re.findall(r"mb3d::[a-z_0-9]+(?:::[A-Za-z_0-9]+)*", rs.read_text(errors="replace")):
                calls[m] = calls.get(m, 0) + 1
    behind = int(run("git", "rev-list", "--count", f"{snap}..HEAD", cwd=OURS).strip() or 0)
    return {
        "kind": "copy",
        "snapshot": snap,
        "snapshot_how": how,
        "snapshot_desc": describe(snap),
        "snapshot_version": version_at(snap),
        "behind": behind,
        "kept": kept,
        "changed": changed,
        "newer": newer,
        "new": new,
        "left_out": left_out,
        "calls": calls,
    }


def check_fork(repo, d):
    run("git", "fetch", "-q", str(OURS), "HEAD:refs/watch/ours", cwd=d, ok=True)
    ahead = run("git", "log", "--format=%h %ad %s", "--date=short", "refs/watch/ours..HEAD", cwd=d, ok=True).splitlines()
    behind = run("git", "rev-list", "--count", "HEAD..refs/watch/ours", cwd=d, ok=True).strip()
    files = run("git", "diff", "--stat", "refs/watch/ours...HEAD", cwd=d, ok=True).strip().splitlines()
    return {"kind": "fork", "ahead": ahead, "behind": behind, "diffstat": files[-40:]}


def head_info(d):
    return run("git", "log", "-1", "--format=%H|%ad|%s", "--date=iso", cwd=d).strip().split("|", 2)


def report(results, prev):
    now = datetime.datetime.now().strftime("%Y-%m-%d %H:%M")
    out = [f"# Projects built on mb3d-rs: {now}", ""]
    news = []
    for repo, r in results.items():
        p = prev.get(repo, {})
        out += [f"## {repo}", "", f"Latest commit: `{r['head'][0][:7]}` {r['head'][1]}: {r['head'][2]}", ""]
        log = r.get("log", [])
        old = set(p.get("log", []))
        fresh = [l for l in log if l not in old] if p else []
        if fresh:
            news.append(f"{repo}: {len(fresh)} new commit(s)")
            out += ["**New since the last check:**", ""] + [f"- {l}" for l in fresh] + [""]
        if r["kind"] == "copy":
            out += [
                f"**Our code in it:** taken from our `{r['snapshot'][:7]}` (v{r['snapshot_version']}, {r['snapshot_how']}): {r['snapshot_desc']}. "
                f"We have {r['behind']} commits since.",
                "",
                f"- kept as they are: {len(r['kept'])} files",
                f"- changed: {len(r['changed'])} files",
                f"- newer versions of ours taken in later: {len(r['newer'])} files",
                f"- their own new files: {len(r['new'])}",
                f"- our files left out: {len(r['left_out'])}",
                "",
            ]
            if p and p.get("snapshot") != r["snapshot"]:
                news.append(f"{repo}: now based on our {r['snapshot'][:7]} (v{r['snapshot_version']}), was {p.get('snapshot', '?')[:7]}")
            if r["changed"]:
                out += ["| Changed file | lines added | lines removed |", "|---|---|---|"]
                out += [f"| `{f}` | {a} | {m} |" for f, a, m in sorted(r["changed"], key=lambda x: -(x[1] + x[2]))]
                out += [""]
            if r["newer"]:
                out += ["Taken from later commits of ours:", ""] + [f"- `{f}` ({describe(c)})" for f, c in r["newer"]] + [""]
            if r["new"]:
                out += ["Their new files:", ""] + [f"- `{f}` ({n} lines)" for f, n in r["new"]] + [""]
            if r["left_out"]:
                out += [f"Left out: {', '.join('`'+f+'`' for f in r['left_out'][:30])}" + (" …" if len(r["left_out"]) > 30 else ""), ""]
            if r["calls"]:
                out += ["**Our functions their app calls** (outside the copied engine):", ""]
                out += [f"- `{k}` ({v}×)" for k, v in sorted(r["calls"].items(), key=lambda x: (-x[1], x[0]))] + [""]
            pc = {x[0] for x in p.get("changed", [])}
            pn = {x[0] for x in p.get("new", [])}
            nc = [x[0] for x in r["changed"] if p and x[0] not in pc]
            nn = [x[0] for x in r["new"] if p and x[0] not in pn]
            if nc or nn:
                news.append(f"{repo}: newly changed {len(nc)}, new files {len(nn)}")
        else:
            out += [f"**Fork:** {len(r['ahead'])} commits we do not have; {r['behind']} of ours it lacks.", ""]
            out += [f"- {l}" for l in r["ahead"][:50]] + [""]
            if r["diffstat"]:
                out += ["```", *r["diffstat"], "```", ""]
            if p and len(r["ahead"]) != len(p.get("ahead", [])):
                news.append(f"{repo}: {len(r['ahead'])} own commits (was {len(p.get('ahead', []))})")
    if not prev:
        news.append("first check")
    out[1:1] = ["", "**Changes since the last check:** " + ("; ".join(news) if news else "none"), ""]
    return "\n".join(out), news


def install():
    unit = Path.home() / ".config/systemd/user"
    unit.mkdir(parents=True, exist_ok=True)
    (unit / "mb3d-fork-watch.service").write_text(
        "[Unit]\nDescription=Check projects built on mb3d-rs\n\n[Service]\nType=oneshot\n"
        f"ExecStart=/usr/bin/python3 -I {Path(__file__).resolve()}\n"
    )
    (unit / "mb3d-fork-watch.timer").write_text(
        "[Unit]\nDescription=Check projects built on mb3d-rs once a day\n\n"
        "[Timer]\nOnCalendar=daily\nPersistent=true\nRandomizedDelaySec=1h\n\n[Install]\nWantedBy=timers.target\n"
    )
    run("systemctl", "--user", "daemon-reload")
    run("systemctl", "--user", "enable", "--now", "mb3d-fork-watch.timer")
    print("installed: systemctl --user list-timers mb3d-fork-watch.timer")


def main():
    if "--install" in sys.argv:
        return install()
    HOME.mkdir(parents=True, exist_ok=True)
    state_file = HOME / "state.json"
    prev = json.loads(state_file.read_text()) if state_file.exists() else {}
    results = {}
    for repo in list(COPIES) + [f for f in forks() if f not in COPIES]:
        try:
            d = fetch(repo)
        except Exception as e:
            print(f"{repo}: {e}", file=sys.stderr)
            continue
        r = check_copy(repo, d, COPIES[repo]) if repo in COPIES else check_fork(repo, d)
        r["head"] = head_info(d)
        r["log"] = run("git", "log", "--format=%h %ad %s", "--date=short", "-n", "200", cwd=d).splitlines()
        results[repo] = r
    text, news = report(results, prev)
    stamp = datetime.datetime.now().strftime("%Y-%m-%d_%H%M")
    (HOME / f"report-{stamp}.md").write_text(text)
    (HOME / "latest.md").write_text(text)
    state_file.write_text(json.dumps(results))
    print(text.split("\n")[2])
    print(f"report: {HOME / 'latest.md'}")
    if prev and news:
        subprocess.run(["notify-send", "mb3d-rs forks", "; ".join(news)[:300]], capture_output=True)


if __name__ == "__main__":
    main()
