#!/usr/bin/env python3
"""Structural checks, not a replacement for compilation or behavioral tests.

Resolve Cargo's dependency aliases and workspace inheritance before enforcing the
ACP adapter boundary. Include target-specific tables and every Rust target,
including tests, examples and build scripts. Import checks are supplementary to
manifest checks, not a Rust parser or an OS sandbox.
"""
from pathlib import Path
import json
import re
import subprocess
import sys
import tomllib

DEPENDENCY_TABLES = ('dependencies', 'dev-dependencies', 'build-dependencies')
PROTOCOL_PACKAGE = 'agent-client-protocol'
EXPECTED_ROOT = 'd87e0672a193da01206aceb366b0d8353666c8cf'


def dependency_tables(package):
    for table in DEPENDENCY_TABLES:
        yield table, package.get(table, {})
    for target, value in package.get('target', {}).items():
        for table in DEPENDENCY_TABLES:
            yield f'target.{target}.{table}', value.get(table, {})


def protocol_package(name):
    return name == PROTOCOL_PACKAGE or name.startswith(PROTOCOL_PACKAGE + '-')


def rust_sources(directory):
    for source in directory.rglob('*.rs'):
        if not any(part in {'.git', 'target'} for part in source.relative_to(directory).parts):
            yield source


def audit(root, *, check_git=True):
    root = Path(root).resolve()
    errors = []
    manifest = tomllib.loads((root / 'Cargo.toml').read_text(encoding='utf-8'))
    workspace = manifest['workspace']
    shared = workspace.get('dependencies', {})
    excluded = {path.resolve() for pattern in workspace.get('exclude', []) for path in root.glob(pattern)}
    directories = set()
    for pattern in workspace['members']:
        matches = list(root.glob(pattern))
        if not matches:
            errors.append(f'{pattern}: workspace member missing')
        directories.update(path.resolve() for path in matches if path.resolve() not in excluded)
    for directory in sorted(directories):
        if not directory.is_relative_to(root):
            errors.append('workspace member escapes repository')
            continue
        member = directory.relative_to(root).as_posix()
        path = directory / 'Cargo.toml'
        if not path.is_file():
            errors.append(f'{member}: manifest missing')
            continue
        package = tomllib.loads(path.read_text(encoding='utf-8'))
        adapter = package.get('package', {}).get('name') == 'synara-acp'
        # Cargo can declare custom entry points, so validate those rather than
        # requiring the conventional src directory for every package.
        entries = [directory / 'src/lib.rs', directory / 'src/main.rs']
        if 'lib' in package:
            entries.append(directory / package['lib'].get('path', 'src/lib.rs'))
        entries.extend(directory / binary['path'] for binary in package.get('bin', []) if 'path' in binary)
        if not any(entry.is_file() for entry in entries):
            errors.append(f'{member}: missing crate entry')
        aliases = {'agent_client_protocol', 'agent_client_protocol_schema'}
        for table, dependencies in dependency_tables(package):
            for name, specification in dependencies.items():
                value = specification if isinstance(specification, dict) else {}
                base = directory
                if value.get('workspace') is True:
                    if name not in shared:
                        errors.append(f'{member}: unresolved workspace dependency {name}')
                        continue
                    inherited = shared[name]
                    value = inherited if isinstance(inherited, dict) else {}
                    base = root
                dependency = value.get('package', name)
                if protocol_package(dependency):
                    aliases.add(name.replace('-', '_'))
                    if not adapter:
                        errors.append(f'{member}: ACP dependency outside adapter ({table}.{name} -> {dependency})')
                if 'path' in value and not (base / value['path'] / 'Cargo.toml').is_file():
                    errors.append(f'{member}: missing dependency {name}')
        for source in rust_sources(directory):
            relative = source.relative_to(root).as_posix()
            if source.is_symlink():
                errors.append(f'{relative}: symlinked Rust source is not auditable')
                continue
            text = source.read_text(encoding='utf-8')
            if not adapter:
                for alias in aliases:
                    pattern = rf'(?<![\w])(?:r#)?{re.escape(alias)}\s*(?:::|\bas\b|;)'
                    if re.search(pattern, text):
                        errors.append(f'{relative}: protocol boundary violation')
                        break
            if re.search(r'\b(?:todo|unimplemented)\s*!\s*\(', text):
                errors.append(f'{relative}: unfinished executable path')
    for source in (root / 'crates').rglob('*'):
        if source.suffix in {'.ts', '.tsx', '.js', '.jsx'}:
            errors.append(f'{source.relative_to(root).as_posix()}: non-Rust core source')
    parents = []
    if check_git:
        parents = subprocess.check_output(
            ['git', 'rev-list', '--max-parents=0', 'HEAD'], cwd=root, text=True
        ).splitlines()
        if parents != [EXPECTED_ROOT]:
            errors.append('Expected the single parentless native rewrite root')
    return {'check': 'workspace-structure', 'crates': len(directories), 'root': parents,
            'errors': sorted(set(errors)), 'rust_compilation_performed': False}


def main():
    try:
        report = audit(Path(__file__).resolve().parents[1])
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as error:
        print(json.dumps({'check': 'workspace-structure', 'errors': [str(error)],
                          'rust_compilation_performed': False}, indent=2))
        return 1
    print(json.dumps(report, indent=2))
    return bool(report['errors'])


if __name__ == '__main__':
    sys.exit(main())
