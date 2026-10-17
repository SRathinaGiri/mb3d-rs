#!/usr/bin/env python3
"""Writes the "Updates" section of README.md: every release (newest first)
with its publish time and a table of its commits, plus the commits not
released yet.

Run from the repository: `python3 tools/readme_updates.py`. The release
times come from GitHub (`gh release list`); a tag without a GitHub release
uses the time of its commit. The section sits between the markers
<!-- updates:start --> and <!-- updates:end --> in README.md.
"""
import datetime
import json
import os
import subprocess
import sys

REPO = "thr32423/mb3d-rs"
URL = f"https://github.com/{REPO}"
OPEN = 5  # the newest releases shown open, the rest folded
START, END = "<!-- updates:start -->", "<!-- updates:end -->"


# commits shown with other words than their subject (the user's wish:
# the fix of issue 7 is not named as such in the README)
WORDING = {
    "a97340e": "Big pictures: far less memory (a 15360x15360 picture fits in 32 GB)",
}

def git(*args):
    return subprocess.run(["git", *args], check=True, capture_output=True, text=True).stdout


def published():
    """Tag -> publish time (UTC) of the GitHub releases."""
    gh = "/usr/bin/gh" if os.path.exists("/usr/bin/gh") else "gh"
    try:
        out = subprocess.run(
            [gh, "release", "list", "-R", REPO, "--limit", "1000", "--json", "tagName,publishedAt"],
            check=True, capture_output=True, text=True,
        ).stdout
    except (OSError, subprocess.CalledProcessError) as e:
        print(f"no release times from GitHub ({e}); using the tag commits' times", file=sys.stderr)
        return {}
    return {r["tagName"]: parse(r["publishedAt"]) for r in json.loads(out) if r.get("publishedAt")}


def parse(s):
    return datetime.datetime.fromisoformat(s.replace("Z", "+00:00")).astimezone(datetime.timezone.utc)


def when(t):
    return f"{t.day} {t:%b %Y, %H:%M} UTC"


def commits(rng):
    out = git("log", "--no-merges", "--format=%h%x1f%H%x1f%s", rng)
    rows = []
    for line in out.splitlines():
        short, full, subject = line.split("\x1f")
        if subject.startswith("Version ") and subject[8:9].isdigit():
            subject = "Version number and release notes of this release"
        subject = WORDING.get(short, subject)
        subject = subject.replace("|", "\\|")
        rows.append(f"| [`{short}`]({URL}/commit/{full}) | {subject} |")
    return rows


def table(rows):
    return ["| Commit | What it does |", "|---|---|", *rows]


def main():
    tags = git("tag", "--list", "v*", "--sort=creatordate").split()
    times = published()
    parts = []
    head = tags[-1] if tags else None
    unreleased = commits(f"{head}..HEAD") if head else []
    if unreleased:
        parts += ["### Not released yet", "", *table(unreleased), ""]
    blocks = []
    for i, tag in enumerate(tags):
        rng = f"{tags[i - 1]}..{tag}" if i > 0 else tag
        t = times.get(tag) or parse(git("log", "-1", "--format=%cI", tag).strip())
        note = "" if tag in times else " (tag only, no GitHub release)"
        link = f"[{tag}]({URL}/releases/tag/{tag})" if tag in times else tag
        blocks.append([f"### {link} · {when(t)}{note}", "", *table(commits(rng)), ""])
    blocks.reverse()
    for b in blocks[:OPEN]:
        parts += b
    if len(blocks) > OPEN:
        oldest, newest = tags[0], tags[-OPEN - 1]
        parts += [f"<details><summary>Older releases ({oldest} to {newest})</summary>", ""]
        for b in blocks[OPEN:]:
            parts += b
        parts += ["</details>", ""]
    body = "\n".join(parts).rstrip("\n")

    readme = open("README.md", encoding="utf-8").read()
    if START not in readme:
        sys.exit(f"README.md has no {START} marker")
    a = readme.index(START) + len(START)
    b = readme.index(END)
    readme = readme[:a] + "\n" + body + "\n" + readme[b:]
    open("README.md", "w", encoding="utf-8").write(readme)
    print(f"README.md: {len(tags)} releases, {len(unreleased)} commits not released yet")


if __name__ == "__main__":
    main()
