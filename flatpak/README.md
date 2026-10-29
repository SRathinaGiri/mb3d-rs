# The Flatpak

`io.github.thr32423.mb3d-rs.yml` is the manifest of the Flatpak, as
submitted to Flathub; `cargo-sources.json` lists the crates of Cargo.lock
for its build without network. The Flatpak is built entirely from source,
as Flathub asks: the program, MB3D's data folders from the original
repository, and Intel Open Image Denoise with its CPU device (the card's
devices need the vendors' proprietary toolkits; the other packages ship
Intel's binaries with them, see the release workflow).

## Building and testing it here

Once: the SDK, the Rust extension and the builder, for the user

    flatpak install --user flathub org.flatpak.Builder org.freedesktop.Platform//26.08 org.freedesktop.Sdk//26.08 org.freedesktop.Sdk.Extension.rust-stable//26.08

To build what is committed on a local branch instead of the tagged release,
point the `mb3d-rs` module's source at this folder (`tools/flatpak_local.sh`
writes such a copy of the manifest to the build folder):

    tools/flatpak_local.sh            # builds and installs it for the user, --user
    flatpak run io.github.thr32423.mb3d-rs
    flatpak run --command=mb3d io.github.thr32423.mb3d-rs --help

The checks Flathub runs:

    flatpak run --command=flatpak-builder-lint org.flatpak.Builder manifest flatpak/io.github.thr32423.mb3d-rs.yml
    flatpak run --command=flatpak-builder-lint org.flatpak.Builder appstream assets/io.github.thr32423.mb3d-rs.metainfo.xml
    flatpak run --command=flatpak-builder-lint org.flatpak.Builder repo build/flatpak/repo

## A new release on Flathub

1. Release as usual (the tag `vX.Y.Z`).
2. If Cargo.lock changed: `python3 tools/flatpak_cargo_sources.py`.
3. In the manifest, set the `mb3d-rs` module's `tag` and `commit` to the
   release; add the release to `assets/...metainfo.xml` (`<releases>`) in
   the commit before the tag, so the Flatpak's description names it.
4. Copy the manifest and `cargo-sources.json` to the Flathub repository
   `flathub/io.github.thr32423.mb3d-rs` and open a pull request there; its
   build bot builds it, a merge publishes it.

## The first submission

Flathub takes new apps as a pull request against the `new-pr` branch of
https://github.com/flathub/flathub, with the manifest and
`cargo-sources.json` at the top of the branch (nothing else):

    gh repo fork flathub/flathub --clone=false
    git clone --branch new-pr --single-branch https://github.com/YOUR_USER/flathub.git flathub-submission
    cd flathub-submission
    git checkout -b io.github.thr32423.mb3d-rs
    cp ../mb3d-rs/flatpak/io.github.thr32423.mb3d-rs.yml ../mb3d-rs/flatpak/cargo-sources.json .
    git add . && git commit -m "Add io.github.thr32423.mb3d-rs" && git push -u origin io.github.thr32423.mb3d-rs
    gh pr create --repo flathub/flathub --base new-pr --title "Add io.github.thr32423.mb3d-rs"

A text for the pull request (Flathub's template asks these questions):

    I am the developer of mb3d-rs (https://github.com/thr32423/mb3d-rs),
    a port of Mandelbulb3D, the 3D fractal program, to Linux, Windows and
    macOS with the picture calculated on the graphics card. LGPL-2.1.

    Everything is built from source: the program from its tagged release
    (Rust, the crates of Cargo.lock in cargo-sources.json), the original
    Mandelbulb3D's formulas, maps and parameters from its repository at a
    fixed commit (same licence), and Intel Open Image Denoise 2.5.1 with
    its CPU device, with oneTBB. Open Image Denoise's CPU kernels need
    Intel's ISPC compiler, which is taken as Intel's release binary and
    used only while building, as org.paraview.ParaView does; it is cleaned
    out of the Flatpak.

    Permissions: dri for the card (Vulkan); wayland and fallback-x11;
    filesystem=home because the program has Mandelbulb3D's own file
    dialogs (no portal), opens parameter files, maps and formulas the user
    keeps in folders of their choice, writes pictures, animations and
    meshes there, and its "swarm" shares a folder between computers.
    No network access.

    The app ID follows the repository (io.github.thr32423.mb3d-rs).
The bot builds the app on x86_64 and aarch64 and leaves a link to try it;
a reviewer then asks for changes or merges, after which Flathub creates
the repository `flathub/io.github.thr32423.mb3d-rs` with you as its
maintainer.
