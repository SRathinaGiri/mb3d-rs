#!/bin/sh
# Downloads the latest mb3d-rs release into ~/mb3d-rs (or the folder given
# as the first argument). Running it again updates the programs; files you
# made in the folder (parameters, History, Mandelbulb3D.ini, ...) are kept.
#
#   curl -fsSL https://raw.githubusercontent.com/thr32423/mb3d-rs/master/download.sh | sh
set -e
dir=${1:-"$HOME/mb3d-rs"}
case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)                 asset=linux-x64 ;;
  Darwin-arm64)                 asset=macos-arm64 ;;
  Darwin-x86_64)                asset=macos-x64 ;;
  *) echo "No mb3d-rs release for $(uname -s) $(uname -m)" >&2; exit 1 ;;
esac
mkdir -p "$dir"
curl -fL "https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-$asset.tar.gz" \
  | tar xz -C "$dir" --strip-components=1
if [ "$(uname -s)" = Darwin ]; then xattr -dr com.apple.quarantine "$dir" 2>/dev/null || true; fi
echo "mb3d-rs is in $dir: start $dir/Mandelbulb3D (the windows) or $dir/mb3d (command line)"
