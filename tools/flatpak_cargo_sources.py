#!/usr/bin/env python3
"""The crates of Cargo.lock as Flatpak sources, for the build without
network of the Flatpak (flatpak/io.github.thr32423.mb3d_rs.yml):
    python3 tools/flatpak_cargo_sources.py [Cargo.lock] [-o flatpak/cargo-sources.json]
Every crate from crates.io becomes an archive source (its .crate file,
checked against the SHA-256 of Cargo.lock) unpacked into cargo/vendor, with
the .cargo-checksum.json cargo wants next to it, and the list ends with the
cargo config that points crates.io at that folder.  The same output as
flatpak-builder-tools' flatpak-cargo-generator.py for a lock file without
git dependencies (this project has none; the script stops if it finds one).
Run it again whenever Cargo.lock changes, before a Flathub update.
"""
import json
import sys
import tomllib

CRATES_IO = "https://static.crates.io/crates"
VENDOR = "cargo/vendor"


def main():
    args = sys.argv[1:]
    out = "flatpak/cargo-sources.json"
    if "-o" in args:
        i = args.index("-o")
        out = args[i + 1]
        del args[i:i + 2]
    lock = args[0] if args else "Cargo.lock"
    with open(lock, "rb") as f:
        packages = tomllib.load(f)["package"]
    sources = []
    for p in packages:
        source = p.get("source")
        if source is None:
            continue  # the project itself
        if not source.startswith("registry+"):
            sys.exit(f"{p['name']} {p['version']} comes from {source}: only crates.io crates are handled")
        name, version, sum_ = p["name"], p["version"], p["checksum"]
        sources.append({
            "type": "archive",
            "archive-type": "tar-gzip",
            "url": f"{CRATES_IO}/{name}/{name}-{version}.crate",
            "sha256": sum_,
            "dest": f"{VENDOR}/{name}-{version}",
        })
        sources.append({
            "type": "inline",
            "contents": json.dumps({"package": sum_, "files": {}}),
            "dest": f"{VENDOR}/{name}-{version}",
            "dest-filename": ".cargo-checksum.json",
        })
    sources.append({
        "type": "inline",
        "contents": '[source.vendored-sources]\ndirectory = "cargo/vendor"\n\n[source.crates-io]\nreplace-with = "vendored-sources"\n',
        "dest": "cargo",
        "dest-filename": "config",
    })
    with open(out, "w") as f:
        json.dump(sources, f, indent=4)
        f.write("\n")
    print(f"{out}: {len(sources) // 2} crates")


if __name__ == "__main__":
    main()
