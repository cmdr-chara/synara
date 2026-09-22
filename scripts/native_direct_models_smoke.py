#!/usr/bin/env python3
"""Real GPUI direct model settings, streaming, cancellation and inert restore.

Uses owned loopback HTTP and private Xvfb only. Configuration is entered through
native input, never written through fixture SQL. No paid provider key is used.
"""
import argparse
import json
import os
import sqlite3
import subprocess
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from native_smoke import Scenario, wait_until
from native_integrations_smoke import fill, click
from native_model_draft_smoke import preference, close
from native_navigation_smoke import selection, event_cursor, prompt_finished
from native_studio_settings_smoke import reveal


def paste(s, control, text):
    reveal(s, control)
    s.click_control(control)
    env = {k: os.environ[k] for k in ('PATH', 'LD_LIBRARY_PATH') if k in os.environ}
    env['DISPLAY'] = s.desktop.name
    clipboard = subprocess.Popen(['xclip', '-selection', 'clipboard', '-in', '-quiet'], env=env,
                                 stdin=subprocess.PIPE, stdout=subprocess.DEVNULL,
                                 stderr=subprocess.DEVNULL)
    try:
        clipboard.stdin.write(text.encode())
        clipboard.stdin.close()
        time.sleep(0.15)
        s.desktop.focus()
        s.desktop.key('a', ('Control_L',))
        s.desktop.key('v', ('Control_L',))
        time.sleep(0.3)
        assert s.desktop.copy_input() == text, 'Native input must contain the reviewed JSON exactly'
    finally:
        if clipboard.poll() is None:
            clipboard.terminate()
        try:
            clipboard.wait(timeout=2)
        except subprocess.TimeoutExpired:
            clipboard.kill()
            clipboard.wait(timeout=2)


def binding(s, task):
    return preference(s, 'task-direct-model:' + task)


def run(s):
    requests = []
    stop = threading.Event()

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def do_GET(self):
            assert self.path == '/v1/models'
            assert 'Authorization' not in self.headers
            requests.append(('GET', None))
            body = json.dumps({'data': [{'id': 'fixture'}, {'id': 'second-model'}]}).encode()
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Content-Length', str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_POST(self):
            assert self.path == '/v1/chat/completions'
            size = int(self.headers.get('Content-Length', 0))
            assert 0 < size < 1024 * 1024 and 'Authorization' not in self.headers
            body = json.loads(self.rfile.read(size))
            assert body['stream'] and body['model'] == 'fixture' and 'tools' not in body
            assert all(m['role'] in ('user', 'assistant') for m in body['messages'])
            requests.append(('POST', body))
            hold = 'hold' in json.dumps(body['messages'][-1])
            events = [
                {'choices': [{'index': 0, 'delta': {'content': 'Direct native answer'}, 'finish_reason': None}]},
                {'choices': [{'index': 0, 'delta': {}, 'finish_reason': 'stop'}]},
                {'choices': [], 'usage': {'prompt_tokens': 12, 'completion_tokens': 3}},
            ]
            self.send_response(200)
            self.send_header('Content-Type', 'text/event-stream')
            self.end_headers()
            try:
                self.wfile.write(('data: ' + json.dumps(events[0]) + '\n\n').encode())
                self.wfile.flush()
                if hold:
                    while not stop.wait(0.1):
                        self.wfile.write(b': ping\n\n')
                        self.wfile.flush()
                else:
                    for event in events[1:]:
                        self.wfile.write(('data: ' + json.dumps(event) + '\n\n').encode())
                    self.wfile.write(b'data: [DONE]\n\n')
                    self.wfile.flush()
            except (BrokenPipeError, ConnectionResetError):
                pass

    server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        s.launch()
        task = selection(s)
        baseline = s.events()
        s.desktop.key('6', ('Control_L',))
        fill(s, 'settings-search', 'Direct models')
        s.click_control('direct-models')
        click(s, 'direct-custom')
        value = {'revision': 0, 'providers': [{
            'id': 'fixture-http', 'name': 'Reviewed loopback fixture', 'protocol': 'open_ai_chat',
            'endpoint': f'http://127.0.0.1:{server.server_port}/v1', 'allow_loopback_http': True,
            'requires_key': False, 'models': [{'id': 'fixture', 'name': 'Fixture model',
                                            'capabilities': {'context_window': 8192, 'source': 'Owned fixture metadata'}}],
        }]}
        paste(s, 'direct-config-editor', json.dumps(value, indent=2))
        click(s, 'direct-save')
        wait_until(lambda: (preference(s, 'direct-model-providers-v1') or {}).get('revision') == 1, 'saved direct providers')
        assert requests == [] and s.events() == baseline
        s.checks.append('native-config-review-save-is-inert-and-separate-from-ACP')
        click(s, 'direct-expand')
        click(s, 'direct-first-model')
        assert binding(s, task) is None and requests == []
        s.desktop.screenshot('direct-model-review', window_only=True)
        click(s, 'direct-confirm')
        wait_until(lambda: binding(s, task), 'reviewed task model binding')
        assert requests == [] and s.events() == baseline
        s.checks.append('explicit-model-review-binds-task-without-sending-or-agent-launch')
        s.click_control('settings-back')
        s.prompt('hello direct')
        wait_until(lambda: any(e['type'] == 'prompt_finished' for e in s.events()), 'direct stream completed')
        events = s.events()
        assert any(e.get('text') == 'Direct native answer' for e in events)
        usage = [e['usage'] for e in events if e['type'] == 'usage_changed'][-1]
        assert usage['input_tokens'] == 12 and usage['output_tokens'] == 3 and usage['context_limit'] == 8192
        with sqlite3.connect((s.data / 'native-workspace.sqlite3').as_uri() + '?mode=ro', uri=True) as db:
            assert db.execute('SELECT COUNT(*) FROM sessions').fetchone()[0] == 0
        assert [method for method, _ in requests] == ['POST']
        s.desktop.screenshot('direct-streamed-transcript', window_only=True)
        s.checks.append('real-native-send-stream-usage-transcript-no-ACP-session-or-tools')
        before = event_cursor(s, task)
        s.prompt('hold')
        wait_until(lambda: len(requests) == 2, 'second explicit request')
        wait_until(lambda: any(e.get('text') == 'Direct native answer' for e in s.events()[len(events):]), 'partial streamed answer')
        s.click_control('composer-submit')
        wait_until(lambda: s.task()['state'] == 'failed', 'Stop closes direct request')
        assert not prompt_finished(s, task, before)
        assert len(requests) == 2
        s.checks.append('native-stop-retains-partial-text-without-false-completion-or-retry')
        stop.set()
        before = s.events()
        close(s)
        s.launch(preserve_selection=True)
        assert selection(s) == task and binding(s, task) and s.events() == before and len(requests) == 2
        s.checks.append('direct-binding-and-history-restore-without-network-or-prompt-replay')
        s.desktop.key('6', ('Control_L',))
        fill(s, 'settings-search', 'Direct models')
        s.click_control('direct-models')
        click(s, 'direct-discover')
        reveal(s, 'direct-config-editor')
        assert [m for m, _ in requests] == ['POST', 'POST', 'GET']
        assert len(preference(s, 'direct-model-providers-v1')['providers'][0]['models']) == 1
        click(s, 'direct-save')
        wait_until(lambda: len(preference(s, 'direct-model-providers-v1')['providers'][0]['models']) == 2, 'reviewed discovered identities')
        models = preference(s, 'direct-model-providers-v1')['providers'][0]['models']
        assert models[1]['capabilities']['tools'] == 'unknown'
        assert len(requests) == 3
        s.checks.append('explicit-discovery-is-reviewed-and-never-invents-model-capabilities')
        click(s, 'direct-return-agent')
        click(s, 'direct-confirm')
        wait_until(lambda: binding(s, task) is None, 'explicit return to ACP route')
        s.click_control('settings-back')
        before = event_cursor(s, task)
        s.prompt('hello')
        wait_until(lambda: prompt_finished(s, task, before), 'existing ACP route still works')
        assert len(requests) == 3
        s.checks.append('explicit-return-to-fresh-ACP-session-preserves-both-transcripts')
        s.desktop.screenshot('direct-and-ACP-conversation', window_only=True)
    finally:
        stop.set()
        server.shutdown()
        server.server_close()
        thread.join(timeout=3)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--fixture', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    scenario = Scenario(parser.parse_args())
    result = {'status': 'failed', 'checks': scenario.checks, 'platform': 'Linux/X11/private Xvfb and loopback HTTP'}
    try:
        run(scenario)
        result['status'] = 'passed'
    except BaseException as error:
        result['error'] = str(error)
        if scenario.process and scenario.process.poll() is None:
            scenario.desktop.screenshot('failure')
        raise
    finally:
        scenario.close()
        (scenario.output / 'result.json').write_text(json.dumps(result, indent=2) + '\n')


if __name__ == '__main__':
    main()
