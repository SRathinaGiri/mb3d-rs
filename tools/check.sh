#!/bin/sh
# Runs the build and test steps of the CI workflow (.github/workflows/ci.yml)
# locally, so a push does not find out ten minutes later that, say, the build
# without the graphics card part is broken.
#
#   tools/check.sh                  run the checks
#   tools/check.sh --install-hook   also run them before every `git push`
#                                   that sends changes to the code
#
# The hook skips pushes that change only text (README, docs) and can be
# bypassed once with `git push --no-verify`.
set -e
cd "$(dirname "$0")/.."

if [ "$1" = "--install-hook" ]; then
    hook=$(git rev-parse --git-path hooks)/pre-push
    cat > "$hook" <<'EOF'
#!/bin/sh
# installed by tools/check.sh --install-hook
zero=0000000000000000000000000000000000000000
code=no
while read -r lref lsha rref rsha; do
    [ "$lsha" = "$zero" ] && continue # a deleted branch or tag
    if [ "$rsha" = "$zero" ]; then
        range=$lsha # new on the remote: compare with what it has
        base=$(git merge-base "$lsha" "@{upstream}" 2>/dev/null) && range="$base..$lsha"
    else
        range="$rsha..$lsha"
    fi
    if git diff --name-only $range -- src examples tests build.rs Cargo.toml Cargo.lock .cargo .github 2>/dev/null | grep -q .; then
        code=yes
    fi
done
[ $code = no ] && exit 0
echo "pre-push: the code changed, running tools/check.sh (skip: git push --no-verify)"
exec "$(git rev-parse --show-toplevel)/tools/check.sh"
EOF
    chmod +x "$hook"
    echo "installed $hook"
    exit 0
fi

step() {
    echo "== $1"
    shift
    if ! "$@"; then
        echo "FAILED: $*" >&2
        exit 1
    fi
}
step "Build" cargo build --release
step "Build the renderer alone (no desktop application)" cargo build --release --no-default-features --bin mb3d
step "Test" cargo test --release
step "Build and test with the GPU feature" cargo test --release --features gpu --lib gpu
echo "== all checks passed"
