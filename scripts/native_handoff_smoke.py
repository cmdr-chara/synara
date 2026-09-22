#!/usr/bin/env python3
"""Native reviewed continuation across independent ACP/direct conversations.

All inputs use actual GPUI controls on an owned Xvfb display. SQL is read-only
verification. Local HTTP is an owned fixture, never a vendor account.
"""
import argparse
import json
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from native_smoke import Scenario, wait_until
from native_navigation_smoke import selection, task_count, event_cursor, prompt_finished
from native_model_draft_smoke import preference, close
from native_project_import_smoke import task_events
from native_integrations_smoke import fill, click
from native_direct_models_smoke import paste, binding


def review(s, query):
    click(s, 'handoff-open')
    fill(s, 'handoff-query', query)
    click(s, 'handoff-target', slot=0)
    wait_until(lambda: s.control_bounds('handoff-draft'), 'reviewed continuation editor')
    s.click_control('handoff-draft')
    return s.desktop.copy_input()


def run(s):
    s.launch()
    source = selection(s)
    before = s.prompt('hello')
    s.finished(before)
    fill(s, 'composer-input', 'Preserved source draft')
    wait_until(lambda: (preference(s, 'task-draft:' + source) or {}).get('text') == 'Preserved source draft', 'source draft persisted')
    original = task_events(s, source)
    count = task_count(s)
    text = review(s, 'Fixture beta')
    assert 'Hello from alpha' in text and 'Reviewed continuation' in text
    assert s.control_bounds('handoff-draft')[3] >= 220
    assert task_count(s) == count and task_events(s, source) == original
    s.desktop.screenshot('handoff-review', window_only=True)
    s.checks.append('native-target-review-is-inert-and-context-editor-is-visible')
    edited = text + '\nReview the previous result'
    paste(s, 'handoff-draft', edited)
    click(s, 'handoff-create')
    wait_until(lambda: selection(s) != source, 'new related conversation selected')
    child = selection(s)
    task, events, sessions = task_events(s, child)
    assert task['agent_id'] == 'beta' and task['state'] == 'ready'
    assert task['project_id'] == original[0]['project_id'] and task['working_directory'] == original[0]['working_directory']
    assert events == [] and sessions == 0
    assert binding(s, child) is None
    assert preference(s, 'task-draft:' + child)['text'] == edited
    origin = preference(s, 'thread-origin:' + child)
    assert origin['parent'] == source and origin['kind'] == 'handoff'
    assert task_events(s, source) == original
    s.checks.append('native-confirm-creates-independent-unsent-task-without-cloning-session-or-source-draft')
    s.desktop.screenshot('handoff-unsent-child', window_only=True)
    old_events = s.events()
    close(s)
    s.launch(preserve_selection=True)
    assert selection(s) == child and s.events() == old_events
    assert preference(s, 'task-draft:' + child)['text'] == edited
    assert task_events(s, child)[2] == 0
    s.checks.append('handoff-draft-origin-and-scope-survive-restart-without-execution')
    fill(s, 'composer-input', 'hello')
    before = event_cursor(s, child)
    click(s, 'composer-submit')
    wait_until(lambda: prompt_finished(s, child, before), 'explicit fresh beta completion')
    assert 'Hello from beta' in ''.join(e.get('text', '') for e in task_events(s, child)[1])
    assert task_events(s, source) == original
    s.checks.append('explicit-send-uses-chosen-agent-without-changing-original-history')
    click(s, 'handoff-open-source')
    wait_until(lambda: selection(s) == source, 'original source opened')
    s.click_control('composer-input')
    assert s.desktop.copy_input() == 'Preserved source draft'
    assert task_events(s, source) == original
    s.checks.append('native-origin-link-restores-untouched-original-draft-and-session')

    requests = []
    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass
        def do_POST(self):
            assert self.path == '/v1/chat/completions' and 'Authorization' not in self.headers
            length = int(self.headers.get('Content-Length', 0))
            assert 0 < length < 1024 * 1024
            request = json.loads(self.rfile.read(length))
            assert request['model'] == 'handoff-fixture'
            requests.append(request)
            body = 'data: {"choices":[{"index":0,"delta":{"content":"Direct continuation answer"},"finish_reason":null}]}\n\ndata: {"choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}\n\ndata: [DONE]\n\n'
            self.send_response(200)
            self.send_header('Content-Type', 'text/event-stream')
            self.send_header('Content-Length', str(len(body)))
            self.end_headers()
            self.wfile.write(body.encode())
    server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
    worker = threading.Thread(target=server.serve_forever, daemon=True)
    worker.start()
    try:
        s.desktop.key('6', ('Control_L',))
        fill(s, 'settings-search', 'Direct models')
        s.click_control('direct-models')
        click(s, 'direct-custom')
        value = {'revision': 0, 'providers': [{'id': 'handoff-http', 'name': 'Handoff HTTP', 'protocol': 'open_ai_chat', 'endpoint': f'http://127.0.0.1:{server.server_port}/v1', 'allow_loopback_http': True, 'requires_key': False, 'models': [{'id': 'handoff-fixture', 'name': 'Handoff model', 'capabilities': {'source': 'owned fixture'}}]}]}
        paste(s, 'direct-config-editor', json.dumps(value))
        click(s, 'direct-save')
        wait_until(lambda: preference(s, 'direct-model-providers-v1'), 'direct target configuration')
        s.click_control('settings-back')
        text = review(s, 'Direct: Handoff')
        assert requests == [] and task_events(s, source) == original
        click(s, 'handoff-create')
        wait_until(lambda: selection(s) not in (source, child), 'direct continuation selected')
        direct = selection(s)
        assert binding(s, direct)['selection']['provider_id'] == 'handoff-http'
        assert task_events(s, direct)[1:] == ([], 0)
        assert requests == []
        fill(s, 'composer-input', 'hello direct')
        before = event_cursor(s, direct)
        click(s, 'composer-submit')
        wait_until(lambda: prompt_finished(s, direct, before), 'explicit direct continuation completes')
        assert len(requests) == 1 and task_events(s, direct)[2] == 0
        assert task_events(s, source) == original
        s.desktop.screenshot('handoff-direct-result', window_only=True)
        s.checks.append('explicit-direct-target-binds-only-new-task-and-sends-without-ACP-session-transfer')
    finally:
        server.shutdown()
        server.server_close()
        worker.join(timeout=3)
    close(s)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--fixture', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    s = Scenario(parser.parse_args())
    result = {'status': 'failed', 'checks': s.checks, 'platform': 'Linux/X11/private Xvfb and owned HTTP'}
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
