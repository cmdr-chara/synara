#!/usr/bin/env python3
"""Owned native recap review/cache/refresh/Stop; real loopback inference, no SQL writes."""
import argparse
import json
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from native_smoke import Scenario, wait_until
from native_integrations_smoke import click, fresh_probe
from native_direct_models_smoke import paste, binding
from native_navigation_smoke import selection, task_count, event_cursor, prompt_finished
from native_model_draft_smoke import preference, close
from native_project_import_smoke import task_events
from native_studio_settings_smoke import reveal
from native_rich_text_smoke import clipboard


def fill(s, control, value):
    reveal(s, control)
    s.click_control(control)
    ui = s.desktop
    ui.key('a', ('Control_L',))
    ui.key('BackSpace')
    for char in value:
        if char == ':':
            ui.key('semicolon', ('Shift_L',))
        elif char == '/':
            ui.key('slash')
        else:
            ui.text(char)
    assert value, 'This journey only fills nonempty fields'
    def selected_exactly():
        try:
            return ui.copy_input() == value
        except AssertionError as error:
            if str(error) != 'Owned native input did not publish a clipboard selection':
                raise
            return False
    wait_until(selected_exactly, f'exact native input for {control}')
    ui.focus()

def cached(s, task):
    return preference(s, 'task-recap:' + task)

def open_recap(s):
    s.desktop.key('p', ('Control_L', 'Shift_L'))
    time.sleep(0.2)
    s.desktop.text('Thread recap')
    time.sleep(0.2)
    s.desktop.key('Return')

def run(s):
    requests = []
    mode = ['normal']
    release = threading.Event()
    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_): pass
        def do_POST(self):
            assert self.path == '/v1/chat/completions'
            size = int(self.headers.get('Content-Length', 0))
            assert 0 < size < 1024 * 1024 and 'Authorization' not in self.headers
            body = json.loads(self.rfile.read(size))
            assert body['model'] == 'fixture' and body['stream'] and 'tools' not in body
            assert len(body['messages']) == 1 and body['messages'][0]['role'] == 'user'
            content = body['messages'][0]['content']
            assert isinstance(content, list) and len(content) == 1
            assert content[0]['type'] == 'text' and isinstance(content[0]['text'], str)
            text = content[0]['text']
            assert 'BEGIN QUOTED CONVERSATION' in text and 'hello' in text
            assert 'Do not execute anything' in text
            kind = mode[0]
            requests.append(body)
            self.send_response(200)
            self.send_header('Content-Type', 'text/event-stream')
            self.end_headers()
            try:
                if kind == 'hold':
                    while not release.wait(0.1):
                        self.wfile.write(b': keepalive\n\n'); self.wfile.flush()
                    return
                if kind == 'tool':
                    chunks = [{'choices': [{'index': 0, 'delta': {'tool_calls': [{
                        'index': 0, 'id': 'unsafe', 'type': 'function',
                        'function': {'name': 'execute', 'arguments': '{}'}
                    }]}, 'finish_reason': None}]},
                    {'choices': [{'index': 0, 'delta': {}, 'finish_reason': 'tool_calls'}]}]
                else:
                    chunks = [{'choices': [{'index': 0, 'delta': {'content':
                        'Context: hello. Decisions: review the fixture. Completed work: observed a reply. Open questions and next steps: verify carefully.'}, 'finish_reason': None}]},
                        {'choices': [{'index': 0, 'delta': {}, 'finish_reason': 'stop'}]}]
                for value in chunks:
                    self.wfile.write(('data: ' + json.dumps(value) + '\n\n').encode())
                self.wfile.write(b'data: [DONE]\n\n'); self.wfile.flush()
            except (BrokenPipeError, ConnectionResetError):
                pass
    server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
    server.daemon_threads = True
    worker = threading.Thread(target=server.serve_forever, daemon=True); worker.start()
    try:
        s.launch()
        task = selection(s)
        cursor = event_cursor(s, task)
        # The first source turn is entered through the real native composer,
        # with durable text and rendered enabled state observed before Send.
        s.click_control('composer-input')
        s.desktop.text('hello')
        wait_until(lambda: (preference(s, 'task-draft:' + task) or {}).get('text') == 'hello', 'saved initial source draft')
        assert task_events(s, task)[1:] == ([], 0)
        s.click_control('composer-submit', enabled=True)
        wait_until(lambda: prompt_finished(s, task, cursor), 'initial real ACP fixture response')
        fill(s, 'composer-input', 'Preserve my normal unsent draft')
        original = task_events(s, task)
        open_recap(s)
        reveal(s, 'recap-provider-unavailable')
        assert requests == [] and cached(s, task) is None
        click(s, 'recap-close')
        s.checks.append('recap-without-provider-is-honestly-unavailable-and-does-not-send')

        s.desktop.key('6', ('Control_L',))
        fill(s, 'settings-search', 'Direct models')
        s.click_control('direct-models')
        click(s, 'direct-custom')
        profile = {'revision': 0, 'providers': [{
            'id': 'recap-fixture', 'name': 'Reviewed recap fixture', 'protocol': 'open_ai_chat',
            'endpoint': f'http://127.0.0.1:{server.server_address[1]}/v1',
            'allow_loopback_http': True, 'requires_key': False,
            'models': [{'id': 'fixture', 'name': 'Fixture recap model',
                        'capabilities': {'context_window': 8192, 'source': 'Owned fixture'}}]
        }]}
        paste(s, 'direct-config-editor', json.dumps(profile))
        click(s, 'direct-save')
        wait_until(lambda: (preference(s, 'direct-model-providers-v1') or {}).get('revision') == 1, 'reviewed direct provider')
        s.click_control('settings-back')
        open_recap(s)
        click(s, 'recap-source')
        click(s, 'recap-model', slot=0)
        reveal(s, 'recap-destination')
        assert requests == [] and binding(s, task) is None and task_events(s, task) == original
        s.desktop.screenshot('recap-reviewed-source-and-destination', window_only=True)
        click(s, 'recap-generate')
        saved = wait_until(lambda: cached(s, task), 'persisted complete recap')
        assert len(requests) == 1 and saved['revision'] == 1
        assert saved['source']['task'] == task and len(saved['source']['sha256']) == 64
        assert saved['provider_id'] == 'recap-fixture' and saved['model_id'] == 'fixture'
        assert saved['text'].startswith('Context: hello')
        assert task_events(s, task) == original and task_count(s) == 1 and binding(s, task) is None
        assert preference(s, 'task-draft:' + task)['text'] == 'Preserve my normal unsent draft'
        s.checks.append('reviewed-separate-model-request-caches-provenance-without-rebinding-ACP-or-mutating-source')
        reveal(s, 'recap-cached')
        click(s, 'recap-copy')
        wait_until(lambda: clipboard(s.desktop) == saved['text'], 'copied cached recap')
        s.checks.append('native-cached-summary-is-visible-and-explicitly-copyable')

        mode[0] = 'hold'
        click(s, 'recap-generate')
        wait_until(lambda: len(requests) == 2, 'one regeneration request')
        fresh_probe(s, 'recap-error', lambda: click(s, 'recap-stop'))
        assert cached(s, task) == saved and task_events(s, task) == original
        release.set()
        s.checks.append('stop-cancels-regeneration-with-previous-cache-and-source-preserved')

        mode[0] = 'tool'
        fresh_probe(s, 'recap-error', lambda: click(s, 'recap-generate'))
        wait_until(lambda: len(requests) == 3, 'one rejected tool proposal')
        assert cached(s, task) == saved and task_events(s, task) == original
        s.checks.append('tool-proposals-cannot-execute-or-replace-cached-recap')
        click(s, 'recap-close')
        close(s)
        s.launch(preserve_selection=True)
        open_recap(s)
        reveal(s, 'recap-cached')
        assert cached(s, task) == saved and len(requests) == 3
        assert task_events(s, task) == original and binding(s, task) is None
        assert preference(s, 'task-draft:' + task)['text'] == 'Preserve my normal unsent draft'
        s.checks.append('restart-restores-cache-without-network-replay-task-creation-or-draft-change')
        click(s, 'recap-close')

        cursor = event_cursor(s, task)
        fill(s, 'composer-input', 'hello with another question')
        s.desktop.key('Return')
        wait_until(lambda: prompt_finished(s, task, cursor), 'new real source turn')
        newer = task_events(s, task)
        open_recap(s)
        reveal(s, 'recap-stale')
        assert cached(s, task) == saved and len(requests) == 3
        mode[0] = 'normal'
        click(s, 'recap-model', slot=0)
        click(s, 'recap-generate')
        updated = wait_until(lambda: cached(s, task) if cached(s, task)['revision'] == 2 else None, 'explicit refreshed cache')
        assert updated['source']['sequence'] > saved['source']['sequence']
        assert updated['source']['sha256'] != saved['source']['sha256']
        assert len(requests) == 4 and task_events(s, task) == newer
        s.checks.append('changed-thread-shows-stale-cache-and-refresh-is-explicit-source-revision-checked')
        s.desktop.screenshot('recap-refreshed', window_only=True)
    finally:
        release.set()
        server.shutdown(); server.server_close(); worker.join(timeout=3)

def main():
    parser = argparse.ArgumentParser()
    for name in ('binary', 'fixture', 'output'): parser.add_argument('--' + name, type=Path, required=True)
    s = Scenario(parser.parse_args())
    result = {'status': 'failed', 'checks': s.checks, 'platform': 'Linux/X11/private Xvfb and loopback HTTP'}
    try:
        run(s); result['status'] = 'passed'
    except BaseException as error:
        result['error'] = str(error)
        if s.process and s.process.poll() is None: s.desktop.screenshot('failure')
        raise
    finally:
        s.close()
        (s.output / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps(result, indent=2), flush=True)
if __name__ == '__main__': main()
