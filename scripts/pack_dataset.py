#!/usr/bin/env python3
"""Pack a directory of artefacts into a deterministic bundle."""

import argparse
import hashlib
import json
import pathlib
import zipfile


def compute_sha256(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(8192), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=pathlib.Path, help="root directory to bundle")
    parser.add_argument("out", type=pathlib.Path, help="output zip path")
    parser.add_argument("--submitter", default="community", help="submitter name")
    parser.add_argument("--toolchain", default="asm", help="toolchain string")
    args = parser.parse_args()

    artifacts = []
    for path in sorted(args.root.rglob("*")):
        if not path.is_file():
            continue
        relative = path.relative_to(args.root).as_posix()
        artifacts.append({
            "kind": path.suffix.lstrip("."),
            "path": relative,
            "sha256": compute_sha256(path),
        })

    manifest = {
        "submitter": args.submitter,
        "toolchain": args.toolchain,
        "artifacts": artifacts,
        "metrics": [],
    }

    args.out.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(args.out, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        archive.writestr("manifest.json", json.dumps(manifest, separators=(",", ":")))
        for artifact in artifacts:
            archive.write(args.root / artifact["path"], artifact["path"])

    print(f"bundle written to {args.out}")


if __name__ == "__main__":
    main()
