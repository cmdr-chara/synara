#!/usr/bin/env python3
"""Build a deterministic release-acceptance feed for an already-built Synara package."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

TARGETS = {
    "x86_64-unknown-linux-gnu": ("linux", "x86_64"),
    "aarch64-apple-darwin": ("macos", "aarch64"),
    "x86_64-pc-windows-msvc": ("windows", "x86_64"),
}
MAX_ARTIFACT = 2 * 1024 * 1024 * 1024


def digest(path: Path) -> str:
    value = hashlib.sha256()
    total = 0
    with path.open("rb") as source:
        while chunk := source.read(1024 * 1024):
            total += len(chunk)
            if total > MAX_ARTIFACT:
                raise ValueError("release artifact exceeds acceptance limit")
            value.update(chunk)
    if total == 0:
        raise ValueError("release artifact is empty")
    return value.hexdigest()


def write_new(path: Path, document: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    data = (json.dumps(document, sort_keys=True, indent=2) + "\n").encode("utf-8")
    with path.open("xb") as output:
        output.write(data)


def build(artifact: Path, target: str, version: str, manifest_out: Path, feed_out: Path) -> None:
    if target not in TARGETS:
        raise ValueError("unsupported target")
    if not version or len(version) > 128 or any(ord(ch) < 33 or ord(ch) > 126 for ch in version):
        raise ValueError("invalid release version")
    if not artifact.is_file():
        raise ValueError("release artifact is missing")
    platform, architecture = TARGETS[target]
    artifact_sha = digest(artifact)
    size = artifact.stat().st_size
    manifest = {
        "format_version": 1,
        "release_version": version,
        "min_data_schema": 1,
        "max_data_schema": 1,
        "artifact": {
            "platform": platform,
            "architecture": architecture,
            "byte_length": size,
            "sha256": artifact_sha,
        },
    }
    write_new(manifest_out, manifest)
    manifest_sha = digest(manifest_out)
    feed = {
        "format": "synara-release-feed-v1",
        "release_version": version,
        "target": target,
        "manifest": {
            "file": manifest_out.name,
            "sha256": manifest_sha,
        },
        "artifact": {
            "file": artifact.name,
            "byte_length": size,
            "sha256": artifact_sha,
        },
    }
    write_new(feed_out, feed)


class ReleaseAcceptanceTests(unittest.TestCase):
    def test_feed_and_manifest_bind_exact_artifact(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            artifact = root / "synara.pkg"
            artifact.write_bytes(b"candidate package")
            manifest = root / "manifest.json"
            feed = root / "feed.json"
            build(
                artifact,
                "x86_64-unknown-linux-gnu",
                "0.1.0-acceptance",
                manifest,
                feed,
            )
            decoded_manifest = json.loads(manifest.read_text(encoding="utf-8"))
            decoded_feed = json.loads(feed.read_text(encoding="utf-8"))
            self.assertEqual(decoded_manifest["artifact"]["sha256"], digest(artifact))
            self.assertEqual(decoded_feed["manifest"]["sha256"], digest(manifest))
            self.assertEqual(decoded_feed["artifact"]["sha256"], digest(artifact))
            self.assertEqual(decoded_manifest["artifact"]["platform"], "linux")
            self.assertEqual(decoded_manifest["artifact"]["architecture"], "x86_64")

    def test_no_clobber_unknown_target_and_empty_artifact_fail(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            artifact = root / "candidate"
            artifact.write_bytes(b"x")
            manifest = root / "manifest.json"
            feed = root / "feed.json"
            build(
                artifact,
                "aarch64-apple-darwin",
                "0.1.0",
                manifest,
                feed,
            )
            with self.assertRaises(FileExistsError):
                build(
                    artifact,
                    "aarch64-apple-darwin",
                    "0.1.0",
                    manifest,
                    root / "other-feed.json",
                )
            with self.assertRaises(ValueError):
                build(
                    artifact,
                    "unknown",
                    "0.1.0",
                    root / "other-manifest.json",
                    root / "other-feed.json",
                )
            empty = root / "empty"
            empty.write_bytes(b"")
            with self.assertRaises(ValueError):
                build(
                    empty,
                    "x86_64-pc-windows-msvc",
                    "0.1.0",
                    root / "empty-manifest.json",
                    root / "empty-feed.json",
                )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--artifact", type=Path)
    parser.add_argument("--target", choices=sorted(TARGETS))
    parser.add_argument("--version")
    parser.add_argument("--manifest-out", type=Path)
    parser.add_argument("--feed-out", type=Path)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(ReleaseAcceptanceTests)
        return 0 if unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful() else 1
    for name in ("artifact", "target", "version", "manifest_out", "feed_out"):
        if getattr(args, name) is None:
            parser.error(f"--{name.replace('_', '-')} is required")
    build(args.artifact, args.target, args.version, args.manifest_out, args.feed_out)
    print(args.manifest_out)
    print(args.feed_out)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
