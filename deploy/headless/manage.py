#!/usr/bin/env python3
"""Stage and activate reviewed server bundles. Service/config/database stay operator-owned."""
import argparse
import hashlib
import json
import os
import platform
from pathlib import Path
import re
import shutil
import tempfile


def current(prefix, name):
    link = prefix / name
    if not link.is_symlink():
        if link.exists():
            raise ValueError(f"{name} must be a managed symlink")
        return None
    value = link.resolve(strict=True)
    if value.parent != (prefix / "releases").resolve() or not value.is_dir():
        raise ValueError(f"{name} does not select a managed release")
    return value


def point(prefix, name, target):
    temporary = prefix / f".{name}-{os.getpid()}"
    created = False
    try:
        temporary.symlink_to(target.relative_to(prefix))
        created = True
        os.replace(temporary, prefix / name)
    finally:
        if created:
            temporary.unlink(missing_ok=True)


def install(prefix, source):
    manifest = json.loads((source / "MANIFEST.json").read_text())
    version = manifest.get("version", "")
    if manifest.get("format") != "synara-headless-package-v1" or not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9.+_-]{0,127}", version):
        raise ValueError("invalid reviewed package manifest")
    expected = {"bin/synara-server", "deploy/Caddyfile", "deploy/server.env", "deploy/synara-server.service", "deploy/manage.py"}
    if set(manifest.get("files", {})) != expected:
        raise ValueError("unexpected package contents")
    target = {"x86_64": "x86_64-unknown-linux-gnu", "aarch64": "aarch64-unknown-linux-gnu"}.get(platform.machine())
    if platform.system() != "Linux" or manifest.get("target") != target:
        raise ValueError("package does not match this Linux host")
    contents = {}
    for name, digest in manifest["files"].items():
        file = source / name
        if file.is_symlink() or not file.is_file() or file.resolve().is_relative_to(source) is False:
            raise ValueError("package file escaped its source")
        contents[name] = file.read_bytes()
        if hashlib.sha256(contents[name]).hexdigest() != digest:
            raise ValueError("package digest mismatch")
    releases = prefix / "releases"
    if releases.is_symlink():
        raise ValueError("release storage must be a real directory")
    releases.mkdir(parents=True, exist_ok=True)
    old = current(prefix, "current")
    current(prefix, "previous")
    destination = releases / version
    if destination.exists() or destination.is_symlink():
        raise ValueError("that release already exists; versions are immutable")
    staging = Path(tempfile.mkdtemp(prefix=".staging-", dir=releases))
    try:
        contents["MANIFEST.json"] = (json.dumps(manifest, sort_keys=True, indent=2) + "\n").encode()
        for name in sorted(contents):
            target = staging / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(contents[name])
        (staging / "bin/synara-server").chmod(0o755)
        staging.rename(destination)
        if old is not None:
            point(prefix, "previous", old)
        point(prefix, "current", destination)
    finally:
        if staging.exists():
            shutil.rmtree(staging)
    print(f"Activated {version}. Restart synara-server.service and check /ready.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["install", "rollback"])
    parser.add_argument("--prefix", type=Path, default=Path("/opt/synara-server"))
    parser.add_argument("--source", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--confirm-compatible-data", action="store_true")
    args = parser.parse_args()
    if not args.prefix.is_absolute() or args.prefix.is_symlink():
        parser.error("prefix must be an absolute real directory")
    prefix = args.prefix.resolve()
    prefix.mkdir(parents=True, exist_ok=True)
    # Serialize activation across concurrent administrators/processes.
    import fcntl
    with (prefix / ".activation.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        if args.action == "install":
            install(prefix, args.source.resolve(strict=True))
        else:
            if not args.confirm_compatible_data:
                parser.error("rollback requires a compatible database or a reviewed backup restore plan")
            old, previous = current(prefix, "current"), current(prefix, "previous")
            if old is None or previous is None:
                parser.error("current and previous releases are required")
            point(prefix, "current", previous)
            point(prefix, "previous", old)
            print("Previous release activated. Restart the service and check /ready.")


if __name__ == "__main__":
    main()
