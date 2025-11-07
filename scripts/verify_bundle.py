#!/usr/bin/env python3
"""Verify a dataset bundle against embedded hashes."""

import argparse
import hashlib
import json
import pathlib
import zipfile


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("bundle", type=pathlib.Path, help="bundle to verify")
    args = parser.parse_args()

    with zipfile.ZipFile(args.bundle, "r") as archive:
        manifest = json.loads(archive.read("manifest.json"))
        for artifact in manifest.get("artifacts", []):
            payload = archive.read(artifact["path"])
            digest = sha256_bytes(payload)
            if digest != artifact["sha256"]:
                raise SystemExit(
                    f"hash mismatch for {artifact['path']}: expected {artifact['sha256']} got {digest}"
                )
    print("bundle verified")


if __name__ == "__main__":
    main()
