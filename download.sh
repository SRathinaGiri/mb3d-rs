#!/bin/sh
# Downloads the latest mb3d-rs release into ~/mb3d-rs (or the folder given
# as the first argument). Running it again updates the programs; files you
# made in the folder (parameters, History, Mandelbulb3D.ini, ...) are kept.
#
#   curl -fsSL https://raw.githubusercontent.com/thr32423/mb3d-rs/master/download.sh | sh
#
# The archive is checked against the release's SHA256SUMS before it is
# unpacked. Then the programs are made available as commands (links in
# ~/.local/bin) and, on Linux, Mandelbulb3D gets an entry in the
# application menu; the man page (man mb3d) and the bash, zsh and fish
# completions of mb3d are installed for your user.
#
#   sh download.sh --uninstall [FOLDER]
#
# removes the links, the menu entry, the man page and the completions again;
# the folder itself is left alone (it may hold your own files): delete it
# yourself if you no longer need it.
set -e

uninstall=
if [ "$1" = "--uninstall" ]; then
  uninstall=1
  shift
fi
dir=${1:-"$HOME/mb3d-rs"}
case "$dir" in /*) ;; *) dir="$PWD/$dir" ;; esac

bin="$HOME/.local/bin"
data="${XDG_DATA_HOME:-$HOME/.local/share}"
conf="${XDG_CONFIG_HOME:-$HOME/.config}"
desktop="$data/applications/mb3d-rs.desktop"
icons="$data/icons/hicolor"
# (and 256x256: mb3d-rs.png)
icon_sizes="16 22 24 32 48 64 128"
man="$data/man/man1/mb3d.1"
bash_c="$data/bash-completion/completions/mb3d"
zsh_c="$data/zsh/site-functions/_mb3d"
fish_c="$conf/fish/completions/mb3d.fish"

# a link of ours: it points into the install folder
remove_link() {
  if [ -L "$1" ]; then
    case "$(readlink "$1")" in "$dir"/*) rm -f "$1" ;; esac
  fi
}

# the desktops see the changed menu entry and icons: the desktop file
# database, KDE's menu cache, and GTK desktops (GNOME, XFCE, ...) look at
# the time of the icon folder
refresh_menus() {
  command -v update-desktop-database > /dev/null 2>&1 && update-desktop-database "$data/applications" 2> /dev/null
  command -v kbuildsycoca6 > /dev/null 2>&1 && kbuildsycoca6 > /dev/null 2>&1
  [ -d "$icons" ] && touch "$icons"
  true
}

if [ -n "$uninstall" ]; then
  remove_link "$bin/mb3d"
  remove_link "$bin/Mandelbulb3D"
  rm -f "$desktop" "$icons/256x256/apps/mb3d-rs.png" "$man" "$bash_c" "$zsh_c" "$fish_c"
  for s in $icon_sizes; do
    rm -f "$icons/${s}x$s/apps/mb3d-rs.png"
  done
  refresh_menus
  echo "Removed the commands, the menu entry, the man page and the completions."
  echo "The programs and your files are still in $dir: delete that folder yourself if you no longer need it."
  exit 0
fi

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)                 asset=linux-x64 ;;
  Darwin-arm64)                 asset=macos-arm64 ;;
  Darwin-x86_64)                asset=macos-x64 ;;
  *) echo "No mb3d-rs release for $(uname -s) $(uname -m)" >&2; exit 1 ;;
esac
if command -v sha256sum > /dev/null 2>&1; then
  sha="sha256sum"
elif command -v shasum > /dev/null 2>&1; then
  sha="shasum -a 256"
else
  echo "Neither sha256sum nor shasum is installed: the download cannot be checked." >&2
  exit 1
fi

# (MB3D_RS_URL: another release folder, for tests)
url="${MB3D_RS_URL:-https://github.com/thr32423/mb3d-rs/releases/latest/download}"
archive="mb3d-rs-$asset.tar.gz"
tmp=$(mktemp -d)
cleanup() {
  rm -f "$tmp/$archive" "$tmp/SHA256SUMS" "$tmp/sum"
  rmdir "$tmp" 2> /dev/null || true
}
trap cleanup EXIT
curl -fL -o "$tmp/$archive" "$url/$archive"
if ! curl -fsSL -o "$tmp/SHA256SUMS" "$url/SHA256SUMS"; then
  echo "Could not download the checksums (SHA256SUMS) of the release: nothing was installed." >&2
  exit 1
fi
grep " $archive\$" "$tmp/SHA256SUMS" > "$tmp/sum" || {
  echo "SHA256SUMS has no entry for $archive: nothing was installed." >&2
  exit 1
}
if ! (cd "$tmp" && $sha -c sum > /dev/null); then
  echo "The download does not match its checksum: nothing was installed." >&2
  exit 1
fi
echo "Checksum OK."

# which version this is (the archive's folder: mb3d-rs-v0.2.46-<target>/),
# and which one the folder has (.mb3d-rs-version, written below)
ver=$(tar tzf "$tmp/$archive" | head -n 1 | sed -n 's|^mb3d-rs-v\([0-9][^-/]*\)-.*|\1|p')
old=$(cat "$dir/.mb3d-rs-version" 2> /dev/null || true)
if [ -n "$ver" ]; then
  if [ "$ver" = "$old" ]; then
    echo "mb3d-rs $ver is installed already in $dir: installing it again."
  elif [ -n "$old" ]; then
    echo "Updating mb3d-rs $old to $ver."
  else
    echo "Installing mb3d-rs $ver."
  fi
fi

mkdir -p "$dir"
tar xzf "$tmp/$archive" -C "$dir" --strip-components=1
[ -n "$ver" ] && printf '%s\n' "$ver" > "$dir/.mb3d-rs-version"
if [ "$(uname -s)" = Darwin ]; then xattr -dr com.apple.quarantine "$dir" 2>/dev/null || true; fi

# the commands (an existing file of the same name that is not a link is kept)
mkdir -p "$bin"
for exe in mb3d Mandelbulb3D; do
  if [ -e "$bin/$exe" ] && [ ! -L "$bin/$exe" ]; then
    echo "Note: $bin/$exe exists and is not a link: left as it is."
  else
    ln -sf "$dir/$exe" "$bin/$exe"
  fi
done

# the man page and the completions (in the archive from 0.2.45 on)
copy() {
  if [ -f "$1" ]; then
    mkdir -p "$(dirname "$2")"
    cp "$1" "$2"
  fi
}
copy "$dir/man/mb3d.1" "$man"
copy "$dir/completions/mb3d.bash" "$bash_c"
copy "$dir/completions/_mb3d" "$zsh_c"
copy "$dir/completions/mb3d.fish" "$fish_c"

# the application menu (Linux desktops)
if [ "$(uname -s)" = Linux ]; then
  # the icon in the sizes of the hicolor theme (the small ones with larger
  # letters): panels, title bars and menus of every desktop take the size
  # they need
  copy "$dir/mb3d-rs.png" "$icons/256x256/apps/mb3d-rs.png"
  for s in $icon_sizes; do
    copy "$dir/mb3d-rs-$s.png" "$icons/${s}x$s/apps/mb3d-rs.png"
  done
  mkdir -p "$(dirname "$desktop")"
  cat > "$desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Mandelbulb3D
GenericName=Fractal renderer
Comment=Mandelbulb3D (mb3d-rs): 3D fractals
Exec="$dir/Mandelbulb3D"
Path=$dir
Icon=mb3d-rs
Terminal=false
Categories=Graphics;3DGraphics;
StartupWMClass=mb3d-rs
EOF
  refresh_menus
fi

echo "mb3d-rs ${ver:-} is in $dir: start Mandelbulb3D (the windows) or mb3d (command line)"
case ":$PATH:" in
  *":$bin:"*) ;;
  *) echo "Add $bin to your PATH to start them by name, e.g. in ~/.profile: export PATH=\"\$HOME/.local/bin:\$PATH\"" ;;
esac
if [ -f "$zsh_c" ]; then
  case "$(basename "${SHELL:-}")" in
    zsh) echo "zsh: completions are in $(dirname "$zsh_c") (add it to fpath before compinit)" ;;
  esac
fi
