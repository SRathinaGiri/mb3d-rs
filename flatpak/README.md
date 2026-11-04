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

## Flathub's rules on AI, read 2026-10-29

Flathub's requirements (https://docs.flathub.org/docs/for-app-authors/requirements,
"Generative AI policy") and the AGENTS.md of flathub/flathub say:

- AI tools or agents must not open or automate submission pull requests,
  nor write their commit messages, descriptions, review comments or replies.
- Flathub manifests must not contain AI-generated or AI-assisted content;
  disclosure does not exempt them.
- AI-generated code in the application itself must be disclosed, with
  the affected parts and their extent; reviewers may reject on that
  ground alone.

The manifest and the files in this folder were written with an AI
assistant, so they cannot be submitted as they are. A submission has to be
the maintainer's own work, done by hand, with the application's
AI-written parts disclosed in the pull request's checklist.
