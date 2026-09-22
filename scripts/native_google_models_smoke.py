#!/usr/bin/env python3
"""Native third-family setup/discovery/stream/schema/restart journey.

Only isolated data, a private X11 display and owned loopback HTTP are used.
SQL is read-only. No Google credentials, provider services or account claims.
"""
import argparse
import json
import sqlite3
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlsplit, parse_qs
from native_smoke import Scenario, wait_until
from native_integrations_smoke import fill, click
from native_direct_models_smoke import paste, binding
from native_model_draft_smoke import preference, close
from native_navigation_smoke import selection, event_cursor, prompt_finished


def run(s):
    requests = []
    stop = threading.Event()
    failures = []

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def do_GET(self):
            parsed = urlsplit(self.path)
            query = parse_qs(parsed.query)
            if parsed.path != '/v1beta/models' or query.get('pageSize') != ['1000']:
                failures.append('unexpected discovery URL')
            requests.append(('GET', self.path))
            second = 'pageToken' in query
            if second and query['pageToken'] != ['opaque&second']:
                failures.append('opaque page token changed')
            body = {'models': [{'name': 'models/second' if second else 'models/fixture',
                                'displayName': 'Second' if second else 'Owned fixture',
                                'supportedGenerationMethods': ['generateContent'],
                                'inputTokenLimit': 8192, 'outputTokenLimit': 4096}]}
            if not second:
                body['nextPageToken'] = 'opaque&second'
            payload = json.dumps(body).encode()
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Content-Length', str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)

        def do_POST(self):
            size = int(self.headers.get('Content-Length', 0))
            if not 0 < size < 1024 * 1024:
                failures.append('request size out of bounds')
                self.send_error(400)
                return
            body = json.loads(self.rfile.read(size))
            requests.append(('POST', body))
            if self.path != '/v1beta/models/fixture:streamGenerateContent?alt=sse':
                failures.append('wrong Google route')
            if 'tools' in body or 'x-goog-api-key' in self.headers or 'Authorization' in self.headers:
                failures.append('unexpected tools or credentials in no-key fixture')
            config = body.get('generationConfig', {})
            if config.get('responseMimeType') != 'application/json' or 'responseJsonSchema' not in config:
                failures.append('reviewed schema was not encoded')
            prompt = json.dumps(body['contents'][-1])
            hold = 'hold' in prompt
            ok = 'mismatch' not in prompt
            self.send_response(200)
            self.send_header('Content-Type', 'text/event-stream')
            self.end_headers()
            def event(value):
                self.wfile.write(('data: ' + json.dumps(value) + '\n\n').encode())
                self.wfile.flush()
            try:
                event({'candidates': [{'index': 0, 'content': {'role': 'model', 'parts': [{'text': '{'}]}}]})
                if hold:
                    while not stop.wait(0.1):
                        self.wfile.write(b': ping\n\n')
                        self.wfile.flush()
                else:
                    event({'candidates': [{'index': 0, 'content': {'role': 'model', 'parts': [{'text': '"ok":' + ('true' if ok else 'false') + '}'}]}, 'finishReason': 'STOP'}],
                           'usageMetadata': {'promptTokenCount': 20, 'cachedContentTokenCount': 4, 'candidatesTokenCount': 3, 'thoughtsTokenCount': 2}})
            except (BrokenPipeError, ConnectionResetError):
                pass

    server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        s.launch()
        task = selection(s)
        before = s.events()
        s.desktop.key('6', ('Control_L',))
        fill(s, 'settings-search', 'Direct models')
        s.click_control('direct-models')
        click(s, 'direct-google')
        s.click_control('direct-config-editor')
        template = json.loads(s.desktop.copy_input())
        profile = template['providers'][0]
        assert profile['requires_key'] and not profile['allow_loopback_http']
        assert profile['protocol'] == 'google_generate_content'
        assert profile['models'][0]['capabilities']['tools'] == 'unknown'
        profile.update(endpoint=f'http://127.0.0.1:{server.server_port}/v1beta', requires_key=False, allow_loopback_http=True)
        profile['models'] = [{'id': 'fixture', 'name': 'Owned schema fixture', 'capabilities': {
            'structured_output': 'supported', 'source': 'Owned fixture metadata', 'context_window': 8192}}]
        paste(s, 'direct-config-editor', json.dumps(template, indent=2))
        click(s, 'direct-save')
        wait_until(lambda: preference(s, 'direct-model-providers-v1'), 'reviewed Google profile saved')
        assert requests == [] and s.events() == before
        s.checks.append('native-Google-template-requires-secure-key-and-save-remains-inert')
        click(s, 'direct-expand')
        click(s, 'direct-first-model')
        s.click_control('direct-options-editor')
        options = json.loads(s.desktop.copy_input())
        options['output'] = {'type': 'json_schema', 'name': 'reply', 'schema': {
            'type': 'object', 'properties': {'ok': {'type': 'boolean', 'enum': [True]}},
            'required': ['ok'], 'additionalProperties': False}}
        paste(s, 'direct-options-editor', json.dumps(options, indent=2))
        s.desktop.screenshot('google-schema-review', window_only=True)
        click(s, 'direct-confirm')
        wait_until(lambda: binding(s, task), 'explicit Google schema selection')
        assert requests == []
        s.click_control('settings-back')
        s.prompt('good schema')
        wait_until(lambda: prompt_finished(s, task, 0), 'schema-valid Google response')
        usage = [e['usage'] for e in s.events() if e['type'] == 'usage_changed'][-1]
        assert usage['input_tokens'] == 20 and usage['output_tokens'] == 5 and usage['context_limit'] == 8192
        assert len(requests) == 1 and not failures
        s.checks.append('native-Google-SSE-schema-success-and-normalized-usage-without-ACP')
        before = event_cursor(s, task)
        s.prompt('mismatch')
        wait_until(lambda: s.task()['state'] == 'failed', 'local schema mismatch failure')
        assert not prompt_finished(s, task, before)
        errors = [e for e in s.events() if e['type'] == 'error']
        assert 'does not match the reviewed JSON schema' in errors[-1]['message']
        assert len(requests) == 2 and any('false' in e.get('text', '') for e in s.events())
        s.desktop.screenshot('google-schema-mismatch', window_only=True)
        s.checks.append('schema-mismatch-retains-partial-text-without-false-success-or-retry')
        before = event_cursor(s, task)
        s.prompt('hold')
        wait_until(lambda: len(requests) == 3, 'active Google request')
        s.click_control('composer-submit')
        wait_until(lambda: s.task()['state'] == 'failed', 'Google Stop')
        assert not prompt_finished(s, task, before)
        stop.set()
        before = s.events()
        close(s)
        s.launch(preserve_selection=True)
        assert s.events() == before and len(requests) == 3 and binding(s, task)
        with sqlite3.connect((s.data / 'native-workspace.sqlite3').as_uri() + '?mode=ro', uri=True) as db:
            assert db.execute('SELECT COUNT(*) FROM sessions').fetchone()[0] == 0
        s.checks.append('Google-cancellation-and-restart-preserve-history-without-replay-or-session')
        s.desktop.key('6', ('Control_L',))
        fill(s, 'settings-search', 'Direct models')
        s.click_control('direct-models')
        click(s, 'direct-discover')
        wait_until(lambda: len(requests) == 5, 'both discovery pages')
        s.click_control('direct-config-editor')
        discovered = json.loads(s.desktop.copy_input())['providers'][0]['models']
        assert [m['id'] for m in discovered] == ['fixture', 'second']
        assert discovered[1]['capabilities']['tools'] == 'unknown'
        assert discovered[1]['capabilities']['context_window'] == 8192
        assert len(preference(s, 'direct-model-providers-v1')['providers'][0]['models']) == 1
        assert not failures
        s.checks.append('Google-paginated-model-discovery-is-reviewed-and-capability-honest')
        s.desktop.screenshot('google-discovery-review', window_only=True)
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
    s = Scenario(parser.parse_args())
    result = {'status': 'failed', 'checks': s.checks, 'platform': 'Linux/X11/private Xvfb and owned Google-protocol HTTP'}
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
        (s.output / 'result.json').write_text(json.dumps(result, indent=2))
        print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
