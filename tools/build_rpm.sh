#!/bin/sh
# Fedora/RHEL/openSUSE package of a Linux release folder:
#
#   tools/build_rpm.sh RELEASE_FOLDER VERSION OUT_DIR
#
# The same files as tools/build_deb.sh, in the same places (the programs in
# /usr/bin, the shipped folders in /usr/share/mb3d-rs); the result is
# OUT_DIR/mb3d-rs-VERSION-1.x86_64.rpm. Needs rpmbuild (the rpm package on
# Debian/Ubuntu). rpmbuild's tree is put together in OUT_DIR/rpmbuild/ (left
# there to look at; it must not exist yet).
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
src=$(cd "$src" && pwd)
top="$out/rpmbuild"
if [ -e "$top" ]; then
  echo "$top exists already" >&2
  exit 1
fi
# rpm versions have no '-'
version=$(echo "$version" | tr - _)
mkdir -p "$top/SPECS"
cat > "$top/SPECS/mb3d-rs.spec" <<EOF
Name: mb3d-rs
Version: $version
Release: 1
Summary: Mandelbulb3D 3D fractal renderer (Rust port)
License: LGPL-2.1-only
URL: https://github.com/SRathinaGiri/mb3d-rs
# the libraries the programs link are in every glibc system; the rest
# (Vulkan, X11/Wayland) is opened while running, so rpm's search for
# dependencies is not needed
AutoReqProv: no
Requires: glibc >= 2.28
Requires: (libgcc or libgcc_s1)
Recommends: (vulkan-loader or libvulkan1)

%description
The headless mb3d renderer for Linux, with MB3D's formulas, maps and
example parameter files. Renders on the processor and, through Vulkan,
on the graphics card.

%install
s="$src"
r=%{buildroot}
install -Dm755 "\$s/mb3d" "\$r/usr/bin/mb3d"
mkdir -p "\$r/usr/share/mb3d-rs"
for d in M3Formulas M3Maps M3Parameter EM_JIT_M3Formulas Themes Languages examples; do
  [ -d "\$s/\$d" ] && cp -r "\$s/\$d" "\$r/usr/share/mb3d-rs/"
done
install -d "\$r/usr/share/man/man1"
gzip -9n < "\$s/man/mb3d.1" > "\$r/usr/share/man/man1/mb3d.1.gz"
install -Dm644 "\$s/completions/mb3d.bash" "\$r/usr/share/bash-completion/completions/mb3d"
install -Dm644 "\$s/completions/_mb3d" "\$r/usr/share/zsh/site-functions/_mb3d"
install -Dm644 "\$s/completions/mb3d.fish" "\$r/usr/share/fish/vendor_completions.d/mb3d.fish"
install -d "\$r/usr/share/doc/mb3d-rs"
for f in README.md GETTING_STARTED.md README-MB3D.md MB3D-SOURCE.txt CHANGELOG.txt LICENSE; do
  [ -f "\$s/\$f" ] && install -m644 "\$s/\$f" "\$r/usr/share/doc/mb3d-rs/"
done

%files
/usr/bin/mb3d
/usr/share/mb3d-rs
/usr/share/man/man1/mb3d.1.gz
/usr/share/bash-completion/completions/mb3d
/usr/share/zsh/site-functions/_mb3d
/usr/share/fish/vendor_completions.d/mb3d.fish
/usr/share/doc/mb3d-rs
EOF
# the release's programs as they are: not stripped again, no debug package
# or build-id links; xz, which RHEL 8's rpm reads too
rpmbuild -bb --target x86_64-linux \
  --define "_topdir $top" \
  --define "debug_package %{nil}" \
  --define "__os_install_post %{nil}" \
  --define "_build_id_links none" \
  --define "_binary_payload w9.xzdio" \
  "$top/SPECS/mb3d-rs.spec" > "$top/build.log" 2>&1 || { cat "$top/build.log" >&2; exit 1; }
rpm=$(ls "$top"/RPMS/x86_64/mb3d-rs-*.rpm)
mv "$rpm" "$out/"
echo "wrote $out/$(basename "$rpm")"
