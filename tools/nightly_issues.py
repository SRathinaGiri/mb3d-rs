#!/usr/bin/env python3
"""Turns the results of the nightly test run (tools/nightly.py, one
results.json per computer) into GitHub issues with the `gh` command:

* a failure (crash, hang, error) that has no open issue gets one; the same
  failure on another system is added to it. Failures are the same when
  their test, item, kind and message (without numbers) are;
* an open nightly issue whose failure is gone (its test ran and passed on
  every system it failed on) is closed with a note;
* example pictures that changed since the night before get one issue per
  night with the list (someone decides whether the change was meant);
* a system that left no results (its build failed) gets an issue too.

Issues carry the label `nightly`; their body holds hidden markers
(<!-- nightly-sig: ... -->) by which they are found again. Only issues with
that label are ever touched.

    nightly_issues.py RESULTS_DIR --systems linux,windows,macos
                      [--run-url URL] [--dry-run]
"""

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path

LABEL = "nightly"
WORDS = {"crash": "crashes", "timeout": "hangs", "error": "fails"}


def gh(args, dry, quiet=False):
    """Runs gh; with `dry` only shows what it would change (reads run)."""
    if dry and args[:2] != ["issue", "list"]:
        print("DRY gh " + " ".join(a if len(a) < 160 else a[:157] + "..." for a in args))
        return ""
    r = subprocess.run([os.environ.get("GH", "gh")] + args, capture_output=True, text=True)
    if r.returncode != 0 and not quiet:
        print(f"gh {' '.join(args[:3])}: {r.stderr.strip()}", file=sys.stderr)
    return r.stdout


def signature(r):
    lines = [l for l in r["detail"].splitlines() if l.strip()]
    key = next((l for l in lines if "panicked at" in l), lines[-1] if lines else "")
    # (line numbers, sizes and times change from build to build)
    key = re.sub(r"\d+", "#", key.strip())
    text = f"{r['test']}|{r['item']}|{r['status']}|{key}"
    return hashlib.sha1(text.encode()).hexdigest()[:12]


def marker(name, value):
    return f"<!-- nightly-{name}: {value} -->"


def read_marker(body, name):
    m = re.search(rf"<!-- nightly-{name}: (.*?) -->", body or "")
    return m.group(1) if m else None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("results")
    ap.add_argument("--systems", default="linux,windows,macos")
    ap.add_argument("--run-url", default="")
    ap.add_argument("--dry-run", action="store_true")
    a = ap.parse_args()
    dry = a.dry_run
    run = f"[the nightly run]({a.run_url})" if a.run_url else "the nightly run"

    runs = {}
    for f in sorted(Path(a.results).rglob("results.json")):
        d = json.loads(f.read_text())
        runs[d["os"]] = d
    sha = next((d["sha"] for d in runs.values() if d.get("sha")), "")
    short = sha[:7] or "?"

    # tonight's failures, the same one on several systems together
    groups = {}
    passed = set()
    for os_, d in runs.items():
        for r in d["results"]:
            if r["status"] == "ok":
                passed.add((r["test"], r["item"], os_))
                continue
            g = groups.setdefault(signature(r), {"r": r, "systems": []})
            g["systems"].append(os_)
    for os_ in a.systems.split(","):
        if os_ not in runs:
            r = {"test": "build", "item": os_, "status": "error", "detail": f"no results from {os_}: the build or the test run stopped (see the run's log)", "command": "", "seconds": 0}
            groups.setdefault(signature(r), {"r": r, "systems": [os_]})
        else:
            passed.add(("build", os_, os_))

    for name, color, what in [(LABEL, "5319e7", "Found by the nightly test run"), ("picture-change", "fbca04", "A picture changed between two nights")]:
        gh(["label", "create", name, "--color", color, "--description", what, "--force"], dry, quiet=True)
    out = gh(["issue", "list", "--label", LABEL, "--state", "open", "--limit", "300", "--json", "number,title,body"], False)
    issues = json.loads(out) if out.strip() else []
    by_sig = {read_marker(i["body"], "sig"): i for i in issues if read_marker(i["body"], "sig")}

    for sig, g in groups.items():
        r, systems = g["r"], sorted(g["systems"])
        old = by_sig.get(sig)
        if old:
            had = set((read_marker(old["body"], "systems") or "").split(","))
            new = [s for s in systems if s not in had]
            if new:
                body = old["body"].replace(marker("systems", ",".join(sorted(had - {""}))), marker("systems", ",".join(sorted(had | set(new) - {""}))))
                gh(["issue", "edit", str(old["number"]), "--body", body], dry)
                gh(["issue", "comment", str(old["number"]), "--body", f"Also on {', '.join(new)} in {run} of {short}."], dry)
            continue
        title = f"Nightly: {r['test']} {r['item']} {WORDS.get(r['status'], 'fails')}"
        lines = [
            f"Found by {run} of {sha or 'this commit'} on **{', '.join(systems)}**.",
            "",
            f"- test: {r['test']}, {r['item']}",
            f"- result: **{r['status']}** after {r['seconds']} s",
        ]
        if r.get("seed"):
            lines.append(f"- random seed: {r['seed']} (the same command makes the same variations)")
        if r.get("command"):
            lines += ["", "Run again (in a checkout, with the MB3D repository as `mb3d/`):", "", "```", r["command"], "```"]
        if r["detail"]:
            lines += ["", "Output:", "", "```", r["detail"][-3000:], "```"]
        lines += ["", "This issue is closed automatically when the test passes again.", "", marker("sig", sig), marker("item", f"{r['test']}|{r['item']}"), marker("systems", ",".join(systems))]
        gh(["issue", "create", "--title", title[:200], "--label", f"{LABEL},bug", "--body", "\n".join(lines)], dry)

    # failures that are gone
    for i in issues:
        sig = read_marker(i["body"], "sig")
        item = read_marker(i["body"], "item")
        if not sig or not item or sig in groups:
            continue
        test, name = item.split("|", 1)
        systems = [s for s in (read_marker(i["body"], "systems") or "").split(",") if s]
        if systems and all((test, name, s) in passed for s in systems):
            gh(["issue", "close", str(i["number"]), "--comment", f"Passes again in {run} of {short} on {', '.join(systems)}."], dry)

    # changed pictures
    rows = []
    for os_, d in sorted(runs.items()):
        for c in d.get("changes", []):
            how = c.get("note") or f"{c['pixels']} of {c['of']} pixels, by up to {c['max']}"
            rows.append(f"| {os_} | {c['item']} | {how} |")
    if rows:
        lines = [
            f"{len(rows)} example picture(s) differ from the night before ({run} of {sha}).",
            "If the change was meant (a new feature, a fix), close this issue; else it points at a regression.",
            "The pictures of both nights are in the runs' artifacts (`pictures-<system>`).",
            "",
            "| system | picture | difference |",
            "|---|---|---|",
            *rows[:200],
            "",
            marker("kind", "pictures"),
        ]
        gh(["issue", "create", "--title", f"Nightly: {len(rows)} example picture(s) changed ({short})", "--label", f"{LABEL},picture-change", "--body", "\n".join(lines)], dry)

    bad = sum(len(g["systems"]) for g in groups.values())
    print(f"{len(runs)} systems, {bad} failures in {len(groups)} groups, {len(rows)} changed pictures")
    return 0


if __name__ == "__main__":
    sys.exit(main())
