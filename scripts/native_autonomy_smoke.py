#!/usr/bin/env python3
"""Native child-agent workflows, incoming MCP approvals and window-only input.

All windows, services, credentials and projects are independently owned fixtures.
SQLite is inspected read-only. Feature state is changed through GPUI or the
explicit authenticated MCP surface, never through fixture SQL mutation.
"""
import argparse
import base64
import ctypes as C
import http.client
import io
import json
import os
import subprocess
import time
from pathlib import Path
from PIL import Image
from native_smoke import Scenario, wait_until
from native_appsnap_smoke import OwnedWindow
from native_navigation_smoke import selection, task_count
from native_model_draft_smoke import preference, close
from native_integrations_smoke import click, fill, fresh_probe
from native_direct_models_smoke import paste


class KeyEvent(C.Structure):
    _fields_ = [('type', C.c_int), ('serial', C.c_ulong), ('send_event', C.c_int),
                ('display', C.c_void_p), ('window', C.c_ulong), ('root', C.c_ulong),
                ('subwindow', C.c_ulong), ('time', C.c_ulong), ('x', C.c_int), ('y', C.c_int),
                ('x_root', C.c_int), ('y_root', C.c_int), ('state', C.c_uint),
                ('keycode', C.c_uint), ('same_screen', C.c_int)]


class InputEvent(C.Union):
    _fields_ = [('key', KeyEvent), ('pad', C.c_long * 24)]


class InputWindow(OwnedWindow):
    def __init__(self, desktop, name):
        super().__init__(desktop)
        self.rename(name)
        x = desktop.x
        x.XSelectInput.argtypes = [C.c_void_p, C.c_ulong, C.c_long]
        x.XPending.argtypes = [C.c_void_p]
        x.XNextEvent.argtypes = [C.c_void_p, C.POINTER(InputEvent)]
        x.XLookupKeysym.argtypes = [C.POINTER(KeyEvent), C.c_int]
        x.XLookupKeysym.restype = C.c_ulong
        x.XKeysymToString.argtypes = [C.c_ulong]
        x.XKeysymToString.restype = C.c_char_p
        x.XSelectInput(desktop.display, self.window, 1 | 2 | 4 | 8)
        x.XSync(desktop.display, 0)


def inputs(s):
    events = []
    x, display = s.desktop.x, s.desktop.display
    x.XSync(display, 0)
    while x.XPending(display):
        event = InputEvent()
        x.XNextEvent(display, C.byref(event))
        if event.key.type in (2, 3, 4, 5):
            name = None
            if event.key.type in (2, 3):
                value = x.XKeysymToString(x.XLookupKeysym(C.byref(event.key), 0))
                name = value.decode() if value else None
            events.append({'window': event.key.window, 'type': event.key.type,
                           'name': name, 'sent': bool(event.key.send_event),
                           'x': event.key.x, 'y': event.key.y})
    return events


def page(s, query, control):
    s.desktop.key('6', ('Control_L',))
    fill(s, 'settings-search', query)
    s.click_control(control)
    time.sleep(0.4)


def graph(s, root):
    return preference(s, 'task-workflow:' + root)


def clipboard(s):
    result = subprocess.run(['xclip', '-selection', 'clipboard', '-out'],
                            env={k: v for k, v in {**os.environ, 'DISPLAY': s.desktop.name}.items()
                                 if k in ('PATH', 'LD_LIBRARY_PATH', 'DISPLAY')},
                            capture_output=True, timeout=3, check=True)
    assert len(result.stdout) < 4096
    s.desktop.focus()
    return json.loads(result.stdout)


class Client:
    def __init__(self, value):
        config = value['mcpServers']['synara']
        assert config['url'].startswith('http://127.0.0.1:') and config['url'].endswith('/mcp')
        self.address = config['url'][7:-4]
        self.authorization = config['headers']['Authorization']

    def rpc(self, method, params):
        body = json.dumps({'jsonrpc': '2.0', 'id': 1, 'method': method, 'params': params})
        connection = http.client.HTTPConnection(self.address, timeout=8)
        try:
            connection.request('POST', '/mcp', body, headers={
                'Authorization': self.authorization, 'Content-Type': 'application/json',
                'Accept': 'application/json, text/event-stream', 'MCP-Protocol-Version': '2025-11-25'})
            response = connection.getresponse()
            data = response.read(4 * 1024 * 1024)
            assert response.status == 200, 'Scoped fixture endpoint did not accept this request'
            return json.loads(data)
        finally:
            connection.close()

    def tool(self, name, arguments):
        result = self.rpc('tools/call', {'name': name, 'arguments': arguments})
        assert 'result' in result, 'Expected reviewed native MCP tool result'
        return result['result']

    def data(self, name, arguments):
        result = self.tool(name, arguments)
        text = next(item['text'] for item in result['content'] if item['type'] == 'text')
        return json.loads(text)


def approve(s, client, receipt, all_receipts):
    slot = sorted(all_receipts).index(receipt)
    click(s, 'gateway-review', slot=slot)
    click(s, 'gateway-approve')
    return wait_until(lambda: (value if (value := client.data('synara_result', {'request': receipt}))['state'] not in ('pending', 'running') else None), 'native one-shot approval result', 30)


def run(s):
    s.launch()
    root = selection(s)
    page(s, 'workflows', 'workflows')
    click(s, 'workflow-example')
    plan = {'title': 'Native owned workflow', 'concurrency': 1, 'steps': [
        {'title': 'First child', 'agent_id': 'alpha', 'instruction': 'hello', 'depends_on': []},
        {'title': 'Dependent child', 'agent_id': 'beta', 'instruction': 'hello', 'depends_on': [0]},
    ]}
    paste(s, 'workflow-editor', json.dumps(plan, indent=2))
    before = s.events()
    click(s, 'workflow-save')
    value = wait_until(lambda: graph(s, root), 'atomic native child creation')
    assert len(value['steps']) == 2 and s.events() == before and task_count(s) == 3
    assert all(preference(s, 'task-workflow-parent:' + step['task']) == root for step in value['steps'])
    s.checks.append('native-reviewed-dag-creates-unsent-children-with-existing-draft-and-task-owners')
    wait_until(lambda: s.control_bounds('workflow-run'), 'workflow controls')
    click(s, 'workflow-run')
    wait_until(lambda: graph(s, root)['phase'] == 'completed', 'real native child-agent dependency execution', 35)
    assert all(step['state'] == 'completed' and step['attempts'] == 1 for step in graph(s, root)['steps'])
    s.desktop.screenshot('workflow-completed-with-child-usage', window_only=True)
    s.checks.append('native-run-executes-existing-ACP-agents-and-completes-dependent-steps')
    click(s, 'workflow-detach')
    wait_until(lambda: graph(s, root) is None, 'detach graph without deleting children')
    assert task_count(s) == 3

    click(s, 'gateway-enable-external')
    wait_until(lambda: s.control_bounds('gateway-copy', 0), 'native external enrollment')
    click(s, 'gateway-copy', slot=0)
    client = Client(clipboard(s))
    assert client.rpc('initialize', {'protocolVersion': '2025-11-25', 'capabilities': {}, 'clientInfo': {'name': 'Owned native test', 'version': '1'}})['result']['protocolVersion'] == '2025-11-25'
    remote_plan = {'title': 'Client-proposed child', 'steps': [plan['steps'][0]]}
    receipt = client.data('synara_request', {'nonce': 'create-owned', 'operation': {'operation': 'create_workflow', 'spec': remote_plan}})['request']
    assert graph(s, root) is None and task_count(s) == 3
    assert client.data('synara_result', {'request': receipt})['state'] == 'pending'
    s.desktop.screenshot('incoming-request-pending-native-approval', window_only=True)
    result = approve(s, client, receipt, [receipt])
    assert result['state'] == 'completed' and task_count(s) == 4
    assert graph(s, root)['phase'] == 'ready'
    assert client.data('synara_request', {'nonce': 'create-owned', 'operation': {'operation': 'create_workflow', 'spec': remote_plan}})['request'] == receipt
    assert task_count(s) == 4
    s.checks.append('incoming-MCP-request-does-nothing-before-native-approval-and-nonce-replay-cannot-duplicate-children')
    value = graph(s, root)
    second = client.data('synara_request', {'nonce': 'run-owned', 'operation': {'operation': 'run_workflow', 'workflow': value['id'], 'revision': value['revision']}})['request']
    result = approve(s, client, second, [receipt, second])
    assert result['state'] == 'completed' and graph(s, root)['phase'] == 'completed'
    click(s, 'gateway-revoke', slot=0)
    s.checks.append('approved-client-workflow-runs-through-native-owner-and-client-revocation-is-explicit')

    target = InputWindow(s.desktop, '!A Computer input target')
    unrelated = InputWindow(s.desktop, '!Z Unrelated window')
    try:
        page(s, 'Computer', 'computer')
        click(s, 'computer-discover')
        wait_until(lambda: s.control_bounds('computer-select', 0), 'real X11 input target discovery', 25)
        click(s, 'computer-select', slot=0)
        click(s, 'computer-observe')
        wait_until(lambda: s.control_bounds('computer-preview'), 'native observed window preview', 25)
        paste(s, 'computer-input-editor', json.dumps({'action': 'type', 'text': 'hello'}))
        inputs(s)
        click(s, 'computer-review-input')
        assert inputs(s) == [], 'Review must not type into any target'
        s.desktop.screenshot('computer-input-review-before-delivery', window_only=True)
        click(s, 'computer-confirm-input')
        delivered = []
        def typed():
            delivered.extend(inputs(s))
            return len([e for e in delivered if e['type'] == 2]) >= 5
        wait_until(typed, 'window-addressed native input', 25)
        assert [e['name'] for e in delivered if e['type'] == 2] == list('hello')
        assert all(e['window'] == target.window and e['sent'] for e in delivered)
        assert not any(e['window'] == unrelated.window for e in delivered)
        s.checks.append('fresh-frame-native-review-types-only-into-selected-window-never-ambient-focus-or-unrelated-window')
        click(s, 'computer-review-input')
        time.sleep(0.3)
        assert inputs(s) == []
        s.checks.append('consumed-frame-cannot-authorize-a-second-input')
        click(s, 'computer-observe')
        time.sleep(1)
        target.rename('!A Changed target')
        paste(s, 'computer-input-editor', json.dumps({'action': 'key', 'key': 'enter'}))
        click(s, 'computer-review-input')
        click(s, 'computer-confirm-input')
        time.sleep(1)
        assert inputs(s) == []
        s.checks.append('changed-target-identity-refuses-input-and-preserves-other-windows')
        click(s, 'computer-takeover')
        s.desktop.screenshot('computer-control-revoked', window_only=True)
    finally:
        target.close()
        unrelated.close()
    before = s.events()
    close(s)
    s.launch(preserve_selection=True)
    assert graph(s, root)['phase'] == 'completed' and s.events() == before
    page(s, 'Computer', 'computer')
    assert not s.control_bounds('computer-selected') and not s.control_bounds('computer-preview')
    assert not s.control_bounds('gateway-client', 0)
    s.checks.append('restart-restores-workflow-history-without-running-children-or-restoring-client-and-computer-authority')


def main():
    parser = argparse.ArgumentParser()
    for name in ('binary', 'fixture', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    s = Scenario(parser.parse_args())
    result = {'status': 'failed', 'checks': s.checks, 'platform': 'Linux/X11/private Xvfb, owned native windows, ACP and loopback MCP'}
    try:
        run(s)
        result['status'] = 'passed'
    except BaseException as error:
        result['error'] = str(error)
        if s.process and s.process.poll() is None:
            s.desktop.screenshot('failure')
        raise
    finally:
        s.close()
        (s.output / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
