#!/usr/bin/env python3
"""Create deterministic development packages without publishing a release."""
from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
import plistlib
from pathlib import Path
import tarfile
import tempfile
import unittest
import zipfile

TARGETS = {
    "x86_64-unknown-linux-gnu": ("linux-x64", "tar.gz"),
    "aarch64-apple-darwin": ("macos-arm64", "tar.gz"),
    "x86_64-pc-windows-msvc": ("windows-x64", "zip"),
}
MAX_BINARY = 1024 * 1024 * 1024
MAX_INVENTORY = 8 * 1024 * 1024
FIXED_ZIP_TIME = (1980, 1, 1, 0, 0, 0)


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def validate_version(version: str) -> None:
    if not version or len(version) > 128 or any(ord(ch) < 33 or ord(ch) > 126 for ch in version):
        raise ValueError("invalid package version")


def read_bounded(path: Path, limit: int) -> bytes:
    data = path.read_bytes()
    if not data or len(data) > limit:
        raise ValueError(f"invalid or oversized input: {path.name}")
    return data


def manifest(version: str, target: str, binary: bytes, inventory: bytes) -> bytes:
    label, _ = TARGETS[target]
    data = {
        "format": "synara-development-package-v1",
        "version": version,
        "target": target,
        "architecture_label": label,
        "release_status": "development-only",
        "binary_sha256": sha256(binary),
        "dependency_inventory_sha256": sha256(inventory),
        "notes": [
            "This is not a signed production release.",
            "Supported OS minimums and production distribution endpoints are not declared.",
        ],
    }
    return (json.dumps(data, sort_keys=True, indent=2) + "\n").encode()


def macos_info_plist() -> bytes:
    return plistlib.dumps(
        {
            "CFBundleExecutable": "synara-app",
            "CFBundleIdentifier": "dev.synara.app",
            "CFBundleName": "Synara",
            "CFBundlePackageType": "APPL",
            "NSMicrophoneUsageDescription": (
                "Synara uses the microphone only when you explicitly start voice recording."
            ),
        },
        fmt=plistlib.FMT_XML,
        sort_keys=True,
    )


def package_entries(
    version: str, target: str, binary: bytes, inventory: bytes
) -> dict[str, bytes]:
    entries = {
        "DEPENDENCIES.json": inventory,
        "DEVELOPMENT_BUILD.json": manifest(version, target, binary, inventory),
    }
    if target == "aarch64-apple-darwin":
        entries["Synara.app/Contents/MacOS/synara-app"] = binary
        entries["Synara.app/Contents/Info.plist"] = macos_info_plist()
    else:
        executable = (
            "synara-app.exe" if target == "x86_64-pc-windows-msvc" else "synara-app"
        )
        entries[executable] = binary
    return entries


def tar_gz(entries: dict[str, bytes]) -> bytes:
    raw = io.BytesIO()
    with tarfile.open(fileobj=raw, mode="w", format=tarfile.PAX_FORMAT) as archive:
        for name in sorted(entries):
            data = entries[name]
            info = tarfile.TarInfo(name)
            info.size = len(data)
            info.mode = 0o755 if Path(name).name.startswith("synara-app") else 0o644
            info.mtime = 0
            info.uid = 0
            info.gid = 0
            info.uname = ""
            info.gname = ""
            archive.addfile(info, io.BytesIO(data))
    return gzip.compress(raw.getvalue(), compresslevel=9, mtime=0)


def zip_bytes(entries: dict[str, bytes]) -> bytes:
    output = io.BytesIO()
    with zipfile.ZipFile(
        output, mode="w", compression=zipfile.ZIP_DEFLATED, compresslevel=9
    ) as archive:
        for name in sorted(entries):
            info = zipfile.ZipInfo(name, FIXED_ZIP_TIME)
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = (0o755 if name.startswith("synara-app") else 0o644) << 16
            archive.writestr(info, entries[name])
    return output.getvalue()


def build(
    binary_path: Path,
    inventory_path: Path,
    output_dir: Path,
    version: str,
    target: str,
) -> tuple[Path, Path]:
    validate_version(version)
    if target not in TARGETS:
        raise ValueError("unsupported development package target")
    binary = read_bounded(binary_path, MAX_BINARY)
    inventory = read_bounded(inventory_path, MAX_INVENTORY)
    try:
        parsed = json.loads(inventory)
    except json.JSONDecodeError as error:
        raise ValueError("dependency inventory is not JSON") from error
    if parsed.get("format") != "synara-dependency-inventory-v1":
        raise ValueError("unexpected dependency inventory format")
    label, archive_kind = TARGETS[target]
    output_dir.mkdir(parents=True, exist_ok=True)
    base = f"synara-dev-{version}-{label}"
    package = output_dir / f"{base}.{archive_kind}"
    sums = output_dir / f"{base}.sha256"
    if package.exists() or sums.exists():
        raise ValueError("development package output already exists")
    entries = package_entries(version, target, binary, inventory)
    encoded = zip_bytes(entries) if archive_kind == "zip" else tar_gz(entries)
    package.write_bytes(encoded)
    sums.write_text(f"{sha256(encoded)}  {package.name}\n", encoding="ascii")
    return package, sums


class PackageTests(unittest.TestCase):
    def inputs(self, root: Path) -> tuple[Path, Path]:
        binary = root / "app"
        binary.write_bytes(b"synthetic executable bytes")
        inventory = root / "deps.json"
        inventory.write_text(
            json.dumps(
                {
                    "format": "synara-dependency-inventory-v1",
                    "package_count": 0,
                    "external_license_metadata_missing": [],
                    "packages": [],
                    "notes": [],
                }
            ),
            encoding="utf-8",
        )
        return binary, inventory

    def test_tar_package_is_reproducible_and_named_by_architecture(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary, inventory = self.inputs(root)
            one, _ = build(
                binary, inventory, root / "one", "0.1.0-dev", "x86_64-unknown-linux-gnu"
            )
            two, _ = build(
                binary, inventory, root / "two", "0.1.0-dev", "x86_64-unknown-linux-gnu"
            )
            self.assertEqual(one.read_bytes(), two.read_bytes())
            self.assertIn("linux-x64", one.name)

    def test_macos_package_is_an_app_bundle_with_microphone_usage_text(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary, inventory = self.inputs(root)
            package, _ = build(
                binary, inventory, root / "out", "0.1.0-dev", "aarch64-apple-darwin"
            )
            with tarfile.open(package, mode="r:gz") as archive:
                names = sorted(archive.getnames())
                self.assertIn("Synara.app/Contents/MacOS/synara-app", names)
                self.assertIn("Synara.app/Contents/Info.plist", names)
                plist_member = archive.extractfile("Synara.app/Contents/Info.plist")
                self.assertIsNotNone(plist_member)
                metadata = plistlib.loads(plist_member.read())
                self.assertEqual(metadata["CFBundleExecutable"], "synara-app")
                self.assertTrue(metadata["NSMicrophoneUsageDescription"])
                executable = archive.getmember("Synara.app/Contents/MacOS/synara-app")
                self.assertEqual(executable.mode & 0o111, 0o111)

    def test_windows_package_contains_expected_manifest_without_release_claim(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary, inventory = self.inputs(root)
            package, sums = build(
                binary, inventory, root / "out", "0.1.0-dev", "x86_64-pc-windows-msvc"
            )
            with zipfile.ZipFile(package) as archive:
                self.assertEqual(
                    sorted(archive.namelist()),
                    ["DEPENDENCIES.json", "DEVELOPMENT_BUILD.json", "synara-app.exe"],
                )
                metadata = json.loads(archive.read("DEVELOPMENT_BUILD.json"))
                self.assertEqual(metadata["release_status"], "development-only")
                self.assertNotIn("supported_os_minimum", metadata)
            self.assertIn(package.name, sums.read_text(encoding="ascii"))

    def test_no_clobber_and_unknown_targets_fail(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary, inventory = self.inputs(root)
            build(binary, inventory, root / "out", "0.1.0-dev", "aarch64-apple-darwin")
            with self.assertRaises(ValueError):
                build(binary, inventory, root / "out", "0.1.0-dev", "aarch64-apple-darwin")
            with self.assertRaises(ValueError):
                build(binary, inventory, root / "other", "0.1.0-dev", "unknown-target")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--inventory", type=Path)
    parser.add_argument("--output-dir", type=Path)
    parser.add_argument("--version")
    parser.add_argument("--target", choices=sorted(TARGETS))
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(PackageTests)
        return 0 if unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful() else 1
    for name in ("binary", "inventory", "output_dir", "version", "target"):
        if getattr(args, name) is None:
            parser.error(f"--{name.replace('_', '-')} is required")
    package, sums = build(
        args.binary, args.inventory, args.output_dir, args.version, args.target
    )
    print(package)
    print(sums)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
