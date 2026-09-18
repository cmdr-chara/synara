#!/usr/bin/env python3
"""Create a bounded, privacy-safe Cargo dependency/license inventory."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

MAX_PACKAGES = 4096
MAX_TEXT = 1024
FORMAT_VERSION = 1


def bounded(value: object) -> str | None:
    if value is None:
        return None
    text = str(value)
    if not text or len(text) > MAX_TEXT or any(ord(ch) < 32 for ch in text):
        return None
    return text


def source_kind(source: object) -> str:
    text = str(source or "")
    if not text:
        return "workspace"
    if text.startswith("registry+"):
        return "registry"
    if text.startswith("git+"):
        return "git"
    return "other"


def inventory(metadata: dict) -> dict:
    if metadata.get("version") not in (None, FORMAT_VERSION):
        raise ValueError("unsupported cargo metadata format")
    packages = metadata.get("packages")
    members = set(metadata.get("workspace_members") or [])
    if not isinstance(packages, list) or len(packages) > MAX_PACKAGES:
        raise ValueError("invalid or oversized package list")
    rows = []
    missing_external_license = []
    for package in packages:
        if not isinstance(package, dict):
            raise ValueError("invalid package record")
        package_id = package.get("id")
        name = bounded(package.get("name"))
        version = bounded(package.get("version"))
        if not name or not version or not package_id:
            raise ValueError("package identity is incomplete")
        kind = source_kind(package.get("source"))
        license_value = bounded(package.get("license"))
        license_file = bounded(package.get("license_file"))
        workspace = package_id in members
        if not workspace and kind != "workspace" and not (license_value or license_file):
            missing_external_license.append(f"{name} {version}")
        rows.append(
            {
                "name": name,
                "version": version,
                "source_kind": kind,
                "workspace": workspace,
                "license": license_value,
                "license_file_declared": license_file is not None,
            }
        )
    rows.sort(key=lambda row: (row["name"], row["version"], row["source_kind"]))
    return {
        "format": "synara-dependency-inventory-v1",
        "package_count": len(rows),
        "external_license_metadata_missing": sorted(missing_external_license),
        "packages": rows,
        "notes": [
            "Source URLs and local filesystem paths are intentionally omitted.",
            "Workspace package licensing is an owner decision tracked separately in ROADMAP P5.",
            "License metadata is descriptive and does not constitute legal approval.",
        ],
    }


def cargo_metadata(root: Path) -> dict:
    completed = subprocess.run(
        ["cargo", "+1.98.1", "metadata", "--locked", "--format-version", "1"],
        cwd=root,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        timeout=180,
    )
    return json.loads(completed.stdout)


def write_inventory(data: dict, output: Path) -> None:
    if output.exists():
        raise ValueError("output already exists")
    encoded = json.dumps(data, ensure_ascii=False, sort_keys=True, indent=2) + "\n"
    if len(encoded.encode("utf-8")) > 8 * 1024 * 1024:
        raise ValueError("inventory exceeds 8 MiB")
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(encoded, encoding="utf-8")


class InventoryTests(unittest.TestCase):
    def fixture(self) -> dict:
        return {
            "packages": [
                {
                    "id": "path+file:///private/user/project#synara@0.1.0",
                    "name": "synara",
                    "version": "0.1.0",
                    "source": None,
                    "license": None,
                    "license_file": None,
                },
                {
                    "id": "registry+https://example.invalid#index#serde@1.0.0",
                    "name": "serde",
                    "version": "1.0.0",
                    "source": "registry+https://example.invalid/index",
                    "license": "MIT OR Apache-2.0",
                    "license_file": None,
                },
                {
                    "id": "git+https://secret@example.invalid/repo#abc",
                    "name": "fixture-git",
                    "version": "2.0.0",
                    "source": "git+https://secret@example.invalid/repo#abc",
                    "license": None,
                    "license_file": "/private/license",
                },
            ],
            "workspace_members": ["path+file:///private/user/project#synara@0.1.0"],
        }

    def test_inventory_omits_paths_and_source_urls(self) -> None:
        data = inventory(self.fixture())
        encoded = json.dumps(data)
        self.assertNotIn("private/user", encoded)
        self.assertNotIn("secret@example", encoded)
        self.assertEqual(data["package_count"], 3)
        self.assertEqual(data["external_license_metadata_missing"], [])

    def test_missing_external_license_is_reported_not_invented(self) -> None:
        metadata = self.fixture()
        metadata["packages"][1]["license"] = None
        data = inventory(metadata)
        self.assertEqual(data["external_license_metadata_missing"], ["serde 1.0.0"])

    def test_output_is_no_clobber(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "inventory.json"
            write_inventory(inventory(self.fixture()), path)
            with self.assertRaises(ValueError):
                write_inventory(inventory(self.fixture()), path)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path)
    parser.add_argument("--metadata", type=Path)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(InventoryTests)
        return 0 if unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful() else 1
    if args.output is None:
        parser.error("--output is required")
    root = Path(__file__).resolve().parents[1]
    if args.metadata is None:
        metadata = cargo_metadata(root)
    else:
        metadata = json.loads(args.metadata.read_text(encoding="utf-8"))
    data = inventory(metadata)
    write_inventory(data, args.output)
    print(
        f"Dependency inventory PASS: {data['package_count']} packages, "
        f"{len(data['external_license_metadata_missing'])} external license-metadata gaps"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
