#!/bin/sh
# Checks the formulas compiled at run time (the JIT, Cranelift) on this
# computer, e.g. on a Mac, where they are switched off by default:
#
#   tools/jit_check.sh <MB3D folder>     (in the source repository)
#   ./jit_check.sh                       (in the unpacked macOS package)
#
# <MB3D folder> holds M3Formulas and M3Parameter (an MB3D installation, or
# the unpacked mb3d-rs release package; the package's own folder by
# default). The script
#   1. in the repository: builds mb3d and examples/clifcheck with the JIT
#      (on macOS with MB3D_JIT_MACOS=1); in the package: takes the programs
#      next to it,
#   2. compares every formula compiled on its own with its translated
#      version, and the compiled built-in formulas with the Rust ones,
#   3. renders the example files on the CPU with and without the JIT: the
#      pictures must be the same byte for byte; prints both times.
# Everything must say "identical" / "same"; send the output if not.
set -e
here=$(cd "$(dirname "$0")" && pwd)
if [ -x "$here/clifcheck" ] && [ -x "$here/mb3d" ]; then
    # the release package
    dir=${1:-$here}
    bin=$here
    out=${TMPDIR:-/tmp}/mb3d-jit-check
    # a download marked as from the internet: macOS would refuse to start
    # the programs
    if command -v xattr > /dev/null; then xattr -dr com.apple.quarantine "$here" 2> /dev/null || true; fi
else
    dir=${1:?usage: tools/jit_check.sh <MB3D folder with M3Formulas and M3Parameter>}
    cd "$here/.."
    echo "== build"
    MB3D_JIT_MACOS=1 cargo build --release --bin mb3d --example clifcheck
    tgt=${CARGO_TARGET_DIR:-target}
    mkdir -p "$tgt/release/examples"
    bin=$(cd "$tgt/release" && pwd)
    cp "$bin/examples/clifcheck" "$bin/clifcheck"
    out=$tgt/jit_check
fi
echo "== $(uname -sm), $("$bin/mb3d" --version)"
now() { perl -MTime::HiRes=time -e 'printf "%.2f", time'; }
rm -rf "$out/formulas"
mkdir -p "$out/formulas"
# the formulas of MB3D (without this program's own four, which are Rust)
for f in "$dir"/M3Formulas/*.m3f; do
    case "$(basename "$f")" in SphereTree.m3f|UlamSpiral.m3f|PseudoKleinian.m3f|DomeTree.m3f) ;; *) cp "$f" "$out/formulas/" ;; esac
done
export MB3D_JIT=1
echo "== formulas compiled on their own vs translated"
CANON_NAN=1 "$bin/clifcheck" "$out/formulas" --trials 32 | grep -v "^ "
echo "== built-in formulas compiled vs Rust"
"$bin/clifcheck" "$out/formulas" --builtins --trials 1000
echo "== renders with and without the JIT (CPU, 400 px)"
n=0
bad=0
for p in "$dir"/M3Parameter/*.m3p; do
    name=$(basename "$p" .m3p)
    t0=$(now)
    MB3D_JIT=1 MB3D_GPU=0 "$bin/mb3d" render "$p" --cpu -W 400 -o "$out/jit.png" >/dev/null 2>&1 || { echo "FAILED (jit): $name"; bad=$((bad + 1)); continue; }
    t1=$(now)
    MB3D_JIT=0 MB3D_NO_JIT=1 MB3D_GPU=0 "$bin/mb3d" render "$p" --cpu -W 400 -o "$out/nojit.png" >/dev/null 2>&1 || { echo "FAILED (no jit): $name"; continue; }
    t2=$(now)
    if cmp -s "$out/jit.png" "$out/nojit.png"; then same=same; else same=DIFFERENT; bad=$((bad + 1)); fi
    echo "$same  jit $(perl -e "printf '%.2f', $t1 - $t0") s, without $(perl -e "printf '%.2f', $t2 - $t1") s  $name"
    n=$((n + 1))
done
echo "== $n files rendered, $bad different or failed"
