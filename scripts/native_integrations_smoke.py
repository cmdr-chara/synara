#!/usr/bin/env python3
"""Exercise real native integration controls on isolated data and loopback HTTP.

No vendor agent, production credential, repository MCP import, or tools/call is
used. The catalog is mutated only through the native controls, never fixture SQL.
"""
import argparse
import hashlib
import json
import sqlite3
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

from native_smoke import Scenario, wait_until
from native_studio_settings_smoke import fill, reveal
from native_presentation_smoke import resize


def catalog(s):
    with sqlite3.connect((s.data / 'native-workspace.sqlite3').as_uri() + '?mode=ro', uri=True) as db:
        row = db.execute("SELECT data FROM preferences WHERE key='integrations'").fetchone()
        return json.loads(row[0]) if row else {'revision': 0, 'skills': [], 'mcp': []}


def page(s, name):
    s.desktop.key('6', ('Control_L',))
    fill(s, 'settings-search', name)
    s.click_control(name)
    time.sleep(0.25)


def click(s, control, slot=None):
    reveal(s, control)
    s.click_control(control, slot=slot)


def fresh_probe(s, control, operation):
    log = Path(s.log.name)
    before = log.stat().st_size
    operation()
    wait_until(lambda: f'control="{control}"' in log.read_text(errors='replace')[before:], control)


def run(s):
    requests = []
    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass
        def do_POST(self):
            size = int(self.headers.get('Content-Length', '0'))
            assert 0 < size < 32768
            body = json.loads(self.rfile.read(size))
            method = body.get('method')
            requests.append(method)
            assert self.headers['Mcp-Method'] == method
            assert self.headers['MCP-Protocol-Version'] == '2026-07-28'
            assert 'Authorization' not in self.headers
            assert body['params']['_meta']['io.modelcontextprotocol/protocolVersion'] == '2026-07-28'
            if method == 'server/discover':
                result = {'supportedVersions': ['2026-07-28'], 'capabilities': {'tools': {}},
                          '_meta': {'io.modelcontextprotocol/serverInfo': {'name': 'Owned loopback fixture', 'version': '1'}}}
            elif method == 'tools/list':
                result = {'tools': [{'name': 'read_fixture', 'description': 'Discovery only, never executed.', 'inputSchema': {'type': 'object'}}]}
            else:
                raise AssertionError(f'Unexpected network action: {method}')
            data = json.dumps({'jsonrpc': '2.0', 'id': body['id'], 'result': result}).encode()
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Content-Length', str(len(data)))
            self.end_headers()
            self.wfile.write(data)
    server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        s.launch()
        ui = s.desktop
        task = s.task()
        baseline = s.events()
        page(s, 'plugins')
        ui.screenshot('plugins-ownership', window_only=True)
        assert s.events() == baseline and requests == []
        s.checks.append('native-plugins-ownership-no-agent-launch-or-network')

        page(s, 'mcp')
        click(s, 'mcp-add')
        fill(s, 'mcp-name-input', 'Loopback discovery')
        fill(s, 'mcp-endpoint-input', f'http://127.0.0.1:{server.server_port}/mcp')
        click(s, 'mcp-save')
        wait_until(lambda: len(catalog(s)['mcp']) == 1, 'saved MCP')
        item = catalog(s)['mcp'][0]
        assert not item['enabled'] and item['task'] == task['id'] and item['agent_id'] == task['agent_id']
        assert requests == [] and s.events() == baseline
        fresh_probe(s, 'mcp-probe-success', lambda: click(s, 'mcp-test', 1))
        assert requests == ['server/discover', 'tools/list']
        assert not catalog(s)['mcp'][0]['enabled'] and s.events() == baseline
        reveal(s, 'mcp-probe-success')
        ui.screenshot('mcp-probe-evidence', window_only=True)
        s.checks.append('explicit-native-probe-real-HTTP-negotiation-tools-no-execution-or-enable')
        click(s, 'mcp-enable', 1)
        wait_until(lambda: catalog(s)['mcp'][0]['enabled'], 'explicit scoped enable')
        assert requests == ['server/discover', 'tools/list'] and s.events() == baseline
        click(s, 'mcp-edit', 1)
        fill(s, 'mcp-name-input', 'Reviewed connection')
        fill(s, 'mcp-service-input', 'dev.synara.fixture')
        fill(s, 'mcp-account-input', 'reference-only')
        click(s, 'mcp-save')
        wait_until(lambda: catalog(s)['mcp'][0]['name'] == 'Reviewed connection', 'edited MCP')
        assert not catalog(s)['mcp'][0]['enabled']
        assert catalog(s)['mcp'][0]['bearer'] == {'service': 'dev.synara.fixture', 'account': 'reference-only'}
        fresh_probe(s, 'mcp-probe-error', lambda: click(s, 'mcp-test', 1))
        assert requests == ['server/discover', 'tools/list']
        reveal(s, 'mcp-probe-error')
        ui.screenshot('mcp-secret-store-unavailable', window_only=True)
        s.checks.append('native-edit-disables-and-unavailable-secret-store-fails-before-network')
        click(s, 'mcp-remove', 1)
        click(s, 'integration-confirm')
        wait_until(lambda: not catalog(s)['mcp'], 'confirmed local removal')
        assert requests == ['server/discover', 'tools/list']
        s.checks.append('native-remove-explicit-confirmation-no-provider-revocation-claim')

        path = s.project / 'SKILL.md'
        content = '---\nname: Review scoped changes\ndescription: Inspect only the requested diff\nversion: 1\n---\n# Review\nRead before suggesting changes.\n'
        path.write_text(content, encoding='utf-8')
        page(s, 'skills')
        fill(s, 'skill-path', str(path))
        click(s, 'skill-review-path')
        reveal(s, 'skill-approve')
        assert not catalog(s)['skills']
        ui.screenshot('skill-review-origin', window_only=True)
        click(s, 'skill-approve')
        wait_until(lambda: len(catalog(s)['skills']) == 1, 'reviewed skill approval')
        skill = catalog(s)['skills'][0]
        assert not skill['enabled'] and skill['markdown'] == content
        assert skill['origin']['sha256'] == hashlib.sha256(content.encode()).hexdigest()
        assert skill['origin']['path'] == str(path)
        assert s.events() == baseline
        click(s, 'skill-toggle', 1)
        wait_until(lambda: catalog(s)['skills'][0]['enabled'], 'explicit skill enable')
        ui.screenshot('skills-installed', window_only=True)
        fill(s, 'integrations-search', 'no-such-skill-filter')
        ui.screenshot('skills-filter-empty', window_only=True)
        fill(s, 'integrations-search', '')
        click(s, 'skill-insert', 1)
        s.click_control('composer-input')
        inserted = ui.copy_input()
        assert 'Read before suggesting changes.' in inserted and 'Review scoped changes' in inserted
        assert s.events() == baseline
        ui.screenshot('skill-unsent-draft', window_only=True)
        s.checks.append('native-reviewed-file-hash-disabled-install-explicit-enable-search-and-unsent-insert')
        saved = catalog(s)
        ui.request_close()
        wait_until(lambda: s.process.poll() is not None, 'native close')
        assert s.process.returncode == 0
        s.log.close()
        s.log = None
        s.launch(preserve_selection=True)
        assert catalog(s) == saved and s.events() == baseline
        s.click_control('composer-input')
        assert ui.copy_input() == inserted
        page(s, 'skills')
        resize(ui, 1100, 800, s.scale)
        ui.screenshot('skills-restored-narrow', window_only=True)
        click(s, 'skill-remove', 1)
        click(s, 'integration-confirm')
        wait_until(lambda: not catalog(s)['skills'], 'confirmed skill removal')
        assert path.read_text() == content and s.events() == baseline
        s.checks.append('native-restart-library-draft-and-removal-preserve-original-file')
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=3)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--fixture', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--scale', type=float, default=1)
    scenario = Scenario(parser.parse_args())
    result = {'status': 'failed', 'checks': scenario.checks, 'platform': 'Linux/X11/private Xvfb'}
    try:
        run(scenario)
        result['status'] = 'passed'
    except BaseException as error:
        result['error'] = str(error)
        if scenario.process and scenario.process.poll() is None:
            scenario.desktop.screenshot('failure', window_only=True)
        raise
    finally:
        scenario.close()
        (scenario.output / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
