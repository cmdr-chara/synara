#!/usr/bin/env python3
"""Disposable structural fixtures for the protocol boundary checker."""
from pathlib import Path
import tempfile
import unittest

from audit_workspace import audit


class ProtocolBoundaryTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='synara-boundary-')
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.write('Cargo.toml', '[workspace]\nmembers = ["crates/app", "crates/adapter"]\n')
        self.write('crates/app/Cargo.toml', '[package]\nname = "synara-app"\nversion = "0.1.0"\n')
        self.write('crates/adapter/Cargo.toml', '[package]\nname = "synara-acp"\nversion = "0.1.0"\n')
        self.write('crates/app/src/lib.rs', 'pub fn harmless() {}\n')
        self.write('crates/adapter/src/lib.rs', 'pub fn adapter() {}\n')

    def write(self, name, text):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding='utf-8')

    def append(self, name, text):
        path = self.root / name
        path.write_text(path.read_text(encoding='utf-8') + text, encoding='utf-8')

    def errors(self):
        return audit(self.root, check_git=False)['errors']

    def test_clean_boundary_and_nonstandard_adapter_directory(self):
        self.append('crates/adapter/Cargo.toml', '[dependencies]\nprotocol = { package = "agent-client-protocol", version = "=2.1.0" }\n')
        self.write('crates/adapter/src/lib.rs', 'use protocol::schema;\n')
        self.assertEqual(self.errors(), [])

    def test_all_dependency_tables_and_renames(self):
        for table in ('dependencies', 'dev-dependencies', 'build-dependencies',
                      'target.\'cfg(windows)\'.dependencies',
                      'target.\'cfg(unix)\'.dev-dependencies',
                      'target.\'cfg(unix)\'.build-dependencies'):
            with self.subTest(table=table):
                self.write('crates/app/Cargo.toml', f'[package]\nname="synara-app"\nversion="0.1.0"\n[{table}]\nrenamed = {{package="agent-client-protocol",version="=2.1.0"}}\n')
                self.assertTrue(any('ACP dependency outside adapter' in error for error in self.errors()))

    def test_workspace_alias_resolves_real_package(self):
        self.append('Cargo.toml', '[workspace.dependencies]\nwire = {package="agent-client-protocol-schema",version="1"}\n')
        self.append('crates/app/Cargo.toml', '[target.\'cfg(windows)\'.dependencies]\nwire.workspace=true\n')
        self.assertTrue(any('wire -> agent-client-protocol-schema' in error for error in self.errors()))

    def test_unresolved_workspace_dependency_fails_closed(self):
        self.append('crates/app/Cargo.toml', '[dependencies]\nmissing.workspace=true\n')
        self.assertTrue(any('unresolved workspace dependency' in error for error in self.errors()))

    def test_workspace_path_is_relative_to_workspace_not_member(self):
        self.append('Cargo.toml', '[workspace.dependencies]\nlocal = {path="crates/adapter"}\n')
        self.append('crates/app/Cargo.toml', '[dependencies]\nlocal.workspace=true\n')
        self.assertEqual(self.errors(), [])

    def test_direct_path_is_relative_to_member(self):
        self.append('crates/app/Cargo.toml', '[dependencies]\nlocal = {path="../adapter"}\n')
        self.assertEqual(self.errors(), [])
        self.append('crates/app/Cargo.toml', 'absent = {path="../absent"}\n')
        self.assertTrue(any('missing dependency absent' in error for error in self.errors()))

    def test_every_rust_target_is_checked(self):
        for path in ('src/extra.rs', 'tests/escape.rs', 'examples/escape.rs', 'benches/escape.rs', 'build.rs'):
            with self.subTest(path=path):
                self.write('crates/app/' + path, 'use agent_client_protocol :: schema;\n')
                self.assertTrue(any(path in error and 'boundary violation' in error for error in self.errors()))
                (self.root / 'crates/app' / path).unlink()

    def test_raw_identifier_and_use_alias(self):
        for text in ('use r#agent_client_protocol as wire;', 'extern crate agent_client_protocol;',
                     'use agent_client_protocol_schema::SessionId;'):
            with self.subTest(text=text):
                self.write('crates/app/src/lib.rs', text)
                self.assertTrue(any('boundary violation' in error for error in self.errors()))

    def test_similarly_named_identifier_is_not_protocol_import(self):
        self.write('crates/app/src/lib.rs', 'use my_agent_client_protocol::Adapter;')
        self.assertEqual(self.errors(), [])

    def test_custom_entry_and_globbed_members(self):
        self.write('Cargo.toml', '[workspace]\nmembers=["crates/*"]\n')
        (self.root / 'crates/app/src/lib.rs').unlink()
        self.append('crates/app/Cargo.toml', '[lib]\npath="entry.rs"\n')
        self.write('crates/app/entry.rs', 'pub fn custom() {}')
        self.assertEqual(self.errors(), [])

    def test_unfinished_macro_with_whitespace_is_rejected(self):
        self.write('crates/app/src/lib.rs', 'pub fn incomplete() { unimplemented ! (); }')
        self.assertTrue(any('unfinished executable path' in error for error in self.errors()))

    def test_generated_target_directory_is_not_source(self):
        self.write('crates/app/target/generated.rs', 'use agent_client_protocol::schema;')
        self.assertEqual(self.errors(), [])


if __name__ == '__main__':
    unittest.main()
