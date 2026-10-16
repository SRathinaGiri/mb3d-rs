#!/bin/sh
# Debian/Ubuntu package of a Linux release folder:
#
#   tools/build_deb.sh RELEASE_FOLDER VERSION OUT_DIR
#
# RELEASE_FOLDER is the unpacked linux-x64 archive (the programs, MB3D's
# folders, man/, completions/, mb3d-rs*.png, mb3d-rs.desktop); the result is
# OUT_DIR/mb3d-rs_VERSION_amd64.deb. The programs go to /usr/bin and the
# shipped folders to /usr/share/mb3d-rs, where the programs find them
# (src/appdirs.rs: a program below /usr keeps the user's files in
# ~/.config, ~/.local/share and ~/.cache).
#
# Uses dpkg-deb where it is installed, else builds the same archive with
# ar and tar. The package's files are put together in
# OUT_DIR/mb3d-rs_VERSION_amd64/ (left there to look at; it must not exist
# yet).
set -e
src=$1
version=$2
out=$3
if [ ! -x "$src/mb3d" ] || [ -z "$version" ] || [ -z "$out" ]; then
  echo "usage: $0 RELEASE_FOLDER VERSION OUT_DIR" >&2
  exit 2
fi
mkdir -p "$out"
out=$(cd "$out" && pwd)
work="$out/mb3d-rs_${version}_amd64"
if [ -e "$work" ]; then
  echo "$work exists already" >&2
  exit 1
fi
root="$work/root"
share="$root/usr/share"

install -Dm755 "$src/mb3d" "$root/usr/bin/mb3d"
install -Dm755 "$src/Mandelbulb3D" "$root/usr/bin/Mandelbulb3D"
mkdir -p "$share/mb3d-rs"
for d in M3Formulas M3Maps M3Parameter EM_JIT_M3Formulas Themes examples; do
  [ -d "$src/$d" ] && cp -r "$src/$d" "$share/mb3d-rs/"
done
# the icon in the sizes of the hicolor theme (Debian's icon triggers refresh
# the caches of the desktops)
install -Dm644 "$src/mb3d-rs.png" "$share/icons/hicolor/256x256/apps/mb3d-rs.png"
for s in 16 22 24 32 48 64 128; do
  install -Dm644 "$src/mb3d-rs-$s.png" "$share/icons/hicolor/${s}x$s/apps/mb3d-rs.png"
done
install -d "$share/man/man1"
gzip -9n < "$src/man/mb3d.1" > "$share/man/man1/mb3d.1.gz"
install -Dm644 "$src/completions/mb3d.bash" "$share/bash-completion/completions/mb3d"
install -Dm644 "$src/completions/_mb3d" "$share/zsh/vendor-completions/_mb3d"
install -Dm644 "$src/completions/mb3d.fish" "$share/fish/vendor_completions.d/mb3d.fish"
install -Dm644 "$src/mb3d-rs.desktop" "$share/applications/mb3d-rs.desktop"
doc="$share/doc/mb3d-rs"
install -d "$doc"
for f in README.md GETTING_STARTED.md README-MB3D.md MB3D-SOURCE.txt CHANGELOG.txt; do
  [ -f "$src/$f" ] && install -m644 "$src/$f" "$doc/"
done
cat > "$doc/copyright" <<EOF
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: mb3d-rs
Source: https://github.com/thr32423/mb3d-rs

Files: *
Copyright: mb3d-rs authors; Mandelbulb3D by Jesse (and contributors)
License: LGPL-2.1-only
 The Rust port of Mandelbulb3D, under Mandelbulb3D's license, the
 LGPL 2.1 (https://github.com/thargor6/mb3d, see MB3D-SOURCE.txt for the
 commit). On Debian systems the full text is in
 /usr/share/common-licenses/LGPL-2.1.

Files: usr/share/icons/*
Copyright: Mandelbulb3D's icon (m3d128.ico, thargor6/mb3d); modified for
 mb3d-rs (transparent background, the letters "RS" added)
License: LGPL-2.1-only

Files: usr/bin/Mandelbulb3D
Copyright: Liberation fonts (embedded): Red Hat, Inc.
License: OFL-1.1
 The SIL Open Font License 1.1, https://openfontlicense.org
EOF

size=$(du -sk "$root" | cut -f1)
mkdir -p "$work/DEBIAN"
cat > "$work/DEBIAN/control" <<EOF
Package: mb3d-rs
Version: $version
Architecture: amd64
Maintainer: tharsys <tharsys@proton.me>
Installed-Size: $size
Depends: libc6 (>= 2.28), libgcc-s1 | libgcc1
Recommends: libvulkan1, libxkbcommon0, libwayland-client0, libx11-6, libxcursor1, libxrandr2, libxi6
Section: graphics
Priority: optional
Homepage: https://github.com/thr32423/mb3d-rs
Description: Mandelbulb3D 3D fractal renderer (Rust port)
 Mandelbulb3D for Linux: the program with MB3D's windows (Mandelbulb3D)
 and a command line renderer (mb3d), with MB3D's formulas, maps and
 example parameter files. Renders on the processor and, through Vulkan,
 on the graphics card.
EOF
(cd "$root" && find . -type f ! -path './DEBIAN/*' -printf '%P\0' | sort -z | xargs -0 md5sum) > "$work/DEBIAN/md5sums"

deb="$out/mb3d-rs_${version}_amd64.deb"
if command -v dpkg-deb > /dev/null 2>&1; then
  cp -r "$work/DEBIAN" "$root/"
  dpkg-deb --root-owner-group -Zxz --build "$root" "$deb" > /dev/null
else
  (cd "$work/DEBIAN" && tar --owner=0 --group=0 --numeric-owner -cJf "$work/control.tar.xz" ./control ./md5sums)
  (cd "$root" && tar --owner=0 --group=0 --numeric-owner -cJf "$work/data.tar.xz" .)
  printf '2.0\n' > "$work/debian-binary"
  rm -f "$deb"
  (cd "$work" && ar rc "$deb" debian-binary control.tar.xz data.tar.xz)
fi
echo "wrote $deb"
