#!/usr/bin/env python3
"""Create a deterministic Linux headless deployment bundle from a reviewed build."""
import argparse
import gzip
import hashlib
import io
import json
from pathlib import Path
import re
import tarfile


def package(binary, version, target, output):
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9.+_-]{0,127}", version):
        raise ValueError("invalid release version")
    if target not in ("x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"):
        raise ValueError("the service package supports Linux x64 and arm64")
    if not binary.is_file() or not 1 <= binary.stat().st_size <= 1024 ** 3:
        raise ValueError("missing or oversized server binary")
    payload = binary.read_bytes()
    if not payload.startswith(b"\x7fELF"):
        raise ValueError("a Linux ELF server binary is required")
    root = Path(__file__).resolve().parents[1]
    entries = {"bin/synara-server": payload}
    for name in ("Caddyfile", "synara-server.service", "server.env", "manage.py"):
        entries[f"deploy/{name}"] = (root / "deploy/headless" / name).read_bytes()
    manifest = {
        "format": "synara-headless-package-v1", "version": version, "target": target,
        "signing": "unsigned-operator-reviewed",
        "files": {name: hashlib.sha256(data).hexdigest() for name, data in entries.items()},
    }
    entries["MANIFEST.json"] = (json.dumps(manifest, sort_keys=True, indent=2) + "\n").encode()
    output.mkdir(parents=True, exist_ok=True)
    archive = output / f"synara-server-{version}-{target}.tar.gz"
    with archive.open("xb") as raw:
        with gzip.GzipFile(fileobj=raw, mode="wb", filename="", mtime=0) as zipped:
            with tarfile.open(fileobj=zipped, mode="w") as tar:
                for name, data in sorted(entries.items()):
                    info = tarfile.TarInfo(name)
                    info.size = len(data)
                    info.mode = 0o755 if name == "bin/synara-server" else 0o644
                    info.mtime = 0
                    tar.addfile(info, io.BytesIO(data))
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    with archive.with_suffix(archive.suffix + ".sha256").open("x") as receipt:
        receipt.write(f"{digest}  {archive.name}\n")
    print(archive)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--target", required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    package(args.binary, args.version, args.target, args.output_dir)
