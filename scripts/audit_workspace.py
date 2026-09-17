#!/usr/bin/env python3
"""Structural checks only. This is not a replacement for a Rust compiler or cargo tests."""
from pathlib import Path
import json
import subprocess
import sys
import tomllib

root = Path(__file__).resolve().parents[1]
manifest = tomllib.loads((root / 'Cargo.toml').read_text())
errors = []
for member in manifest['workspace']['members']:
    directory = root / member
    package = tomllib.loads((directory / 'Cargo.toml').read_text())
    if not (directory / 'src/lib.rs').exists() and not (directory / 'src/main.rs').exists():
        errors.append(f'{member}: missing crate entry')
    for table in ('dependencies', 'dev-dependencies', 'build-dependencies'):
        for name, value in package.get(table, {}).items():
            if isinstance(value, dict) and 'path' in value:
                if not (directory / value['path'] / 'Cargo.toml').exists():
                    errors.append(f'{member}: missing dependency {name}')
            if name.startswith('agent-client-protocol') and directory.name != 'synara-acp':
                errors.append(f'{member}: ACP dependency outside adapter')
    for source in (directory / 'src').rglob('*.rs'):
        text = source.read_text()
        if directory.name != 'synara-acp' and 'agent_client_protocol::' in text:
            errors.append(f'{source.relative_to(root)}: protocol boundary violation')
        if any(token in text for token in ('todo!(', 'unimplemented!(')):
            errors.append(f'{source.relative_to(root)}: unfinished executable path')
for source in (root / 'crates').rglob('*'):
    if source.suffix in {'.ts', '.tsx', '.js', '.jsx'}:
        errors.append(f'{source.relative_to(root)}: non-Rust core source')
parents = subprocess.check_output(['git', 'rev-list', '--max-parents=0', 'HEAD'], cwd=root, text=True).splitlines()
if len(parents) != 1:
    errors.append('Expected exactly one root commit')
else:
    count = subprocess.check_output(['git', 'rev-list', '--parents', '-n1', parents[0]], cwd=root, text=True).split()
    if len(count) != 1:
        errors.append('Root is not parentless')
report = {'check': 'workspace-structure', 'crates': len(manifest['workspace']['members']), 'root': parents,
          'errors': errors, 'rust_compilation_performed': False}
print(json.dumps(report, indent=2))
sys.exit(bool(errors))
