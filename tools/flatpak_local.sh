#!/bin/sh
# Builds the Flatpak (flatpak/io.github.thr32423.mb3d-rs.yml) from the
# commits of this repository's current branch instead of the tagged release,
# and installs it for the user:
#   tools/flatpak_local.sh [BRANCH]      (default: the current branch)
# The build folder is build/flatpak (its repo for flatpak-builder-lint's
# "repo" check: build/flatpak/repo).  Needs org.flatpak.Builder, the
# freedesktop SDK 25.08 and its rust-stable extension (flatpak/README.md).
set -e
cd "$(dirname "$0")/.."
branch=${1:-$(git rev-parse --abbrev-ref HEAD)}
id=io.github.thr32423.mb3d-rs
mkdir -p build/flatpak
# the manifest with the program's source pointed at this repository
python3 - "$PWD" "$branch" > build/flatpak/$id.yml <<'EOF'
import re, sys
repo, branch = sys.argv[1], sys.argv[2]
m = open("flatpak/io.github.thr32423.mb3d-rs.yml").read()
m = re.sub(r"      - type: git\n        url: https://github.com/thr32423/mb3d-rs.git\n        tag: .*\n        commit: .*\n",
           f"      - type: git\n        path: {repo}\n        branch: {branch}\n", m)
m = m.replace("      - cargo-sources.json", f"      - {repo}/flatpak/cargo-sources.json")
sys.stdout.write(m)
EOF
grep -q "path: $PWD" build/flatpak/$id.yml || { echo "the manifest's mb3d-rs source was not found" >&2; exit 1; }
# (the screenshots mirrored and committed as Flathub's build does, for
# flatpak-builder-lint's "repo" check)
flatpak run org.flatpak.Builder --user --install --force-clean --ccache \
  --mirror-screenshots-url=https://dl.flathub.org/media/ \
  --repo=build/flatpak/repo --state-dir=build/flatpak/state build/flatpak/build "build/flatpak/$id.yml"
echo "installed: flatpak run $id"
