#!/usr/bin/env python3
"""Native Anthropic Messages direct-model interoperability journey.

Uses the real GPUI route with owned loopback HTTP and private Xvfb. No provider
credential is supplied. The fixture validates Anthropic request shape, headers,
stream completion, usage accounting, cancellation, restart and return to ACP.
"""
import argparse
import json
import sqlite3
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

from native_smoke import Scenario, wait_until
from native_integrations_smoke import fill, click
from native_direct_models_smoke import paste, binding
from native_model_draft_smoke import preference, close
from native_navigation_smoke import selection, event_cursor, prompt_finished
from native_studio_settings_smoke import reveal


def run(s):
    requests = []
    stop = threading.Event()

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def do_POST(self):
            assert self.path == '/v1/messages'
            size = int(self.headers.get('Content-Length', 0))
            assert 0 < size < 1024 * 1024
            assert 'x-api-key' not in self.headers
            assert self.headers.get('anthropic-version') == '2023-06-01'
            body = json.loads(self.rfile.read(size))
            assert body['stream'] and body['model'] == 'fixture'
            assert body['max_tokens'] == 1024 and 'tools' not in body
            assert all(message['role'] in ('user', 'assistant') for message in body['messages'])
            requests.append(body)
            hold = 'hold' in json.dumps(body['messages'][-1])
            events = [
                {'type': 'message_start', 'message': {'usage': {'input_tokens': 7, 'output_tokens': 0}}},
                {'type': 'content_block_delta', 'index': 0,
                 'delta': {'type': 'text_delta', 'text': 'Anthropic native answer'}},
            ]
            self.send_response(200)
            self.send_header('Content-Type', 'text/event-stream')
            self.end_headers()

            def emit(value):
                self.wfile.write(('data: ' + json.dumps(value) + '\n\n').encode())
                self.wfile.flush()

            try:
                for event in events:
                    emit(event)
                if hold:
                    while not stop.wait(0.1):
                        self.wfile.write(b': ping\n\n')
                        self.wfile.flush()
                else:
                    emit({'type': 'message_delta',
                          'delta': {'stop_reason': 'end_turn'},
                          'usage': {'output_tokens': 4}})
                    emit({'type': 'message_stop'})
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
        profile = {'revision': 0, 'providers': [{
            'id': 'anthropic-fixture',
            'name': 'Reviewed Anthropic Messages fixture',
            'protocol': 'anthropic_messages',
            'endpoint': f'http://127.0.0.1:{server.server_port}/v1',
            'allow_loopback_http': True,
            'requires_key': False,
            'models': [{
                'id': 'fixture',
                'name': 'Anthropic fixture model',
                'capabilities': {
                    'context_window': 8192,
                    'source': 'Owned fixture metadata',
                },
            }],
        }]}
        paste(s, 'direct-config-editor', json.dumps(profile, indent=2))
        s.desktop.screenshot('anthropic-provider-review', window_only=True)
        click(s, 'direct-save')
        wait_until(
            lambda: (preference(s, 'direct-model-providers-v1') or {}).get('revision') == 1,
            'saved Anthropic profile',
        )
        assert requests == [] and s.events() == baseline
        s.checks.append('anthropic-profile-review-save-is-inert')

        click(s, 'direct-expand')
        click(s, 'direct-first-model')
        click(s, 'direct-history-current')
        click(s, 'direct-output-1024')
        reveal(s, 'direct-options-editor')
        click(s, 'direct-confirm')
        wait_until(lambda: binding(s, task), 'reviewed Anthropic route binding')
        assert binding(s, task)['selection']['history_turns'] == 0
        assert binding(s, task)['selection']['max_output_tokens'] == 1024
        assert requests == []
        s.click_control('settings-back')

        before = event_cursor(s, task)
        s.prompt('hello anthropic')
        wait_until(lambda: prompt_finished(s, task, before), 'Anthropic stream completed')
        events = s.events()
        assert any(event.get('text') == 'Anthropic native answer' for event in events)
        usage = [event['usage'] for event in events if event['type'] == 'usage_changed'][-1]
        assert usage['input_tokens'] == 7
        assert usage['output_tokens'] == 4
        assert usage['context_limit'] == 8192
        assert len(requests) == 1
        with sqlite3.connect((s.data / 'native-workspace.sqlite3').as_uri() + '?mode=ro', uri=True) as db:
            assert db.execute('SELECT COUNT(*) FROM sessions').fetchone()[0] == 0
        s.desktop.screenshot('anthropic-streamed-transcript', window_only=True)
        s.checks.append('anthropic-native-stream-usage-and-no-ACP-session')

        before = event_cursor(s, task)
        s.prompt('hold')
        wait_until(lambda: len(requests) == 2, 'active Anthropic request')
        wait_until(
            lambda: any(
                event.get('text') == 'Anthropic native answer'
                for event in s.events()[len(events):]
            ),
            'Anthropic partial output',
        )
        s.click_control('composer-submit')
        wait_until(lambda: s.task()['state'] == 'failed', 'Anthropic Stop')
        assert not prompt_finished(s, task, before)
        assert len(requests) == 2
        s.checks.append('anthropic-stop-retains-partial-output-without-retry')
        stop.set()

        saved_events = s.events()
        close(s)
        s.launch(preserve_selection=True)
        assert selection(s) == task
        assert binding(s, task)
        assert s.events() == saved_events
        assert len(requests) == 2
        s.checks.append('anthropic-route-and-history-restore-without-replay')

        s.desktop.key('6', ('Control_L',))
        fill(s, 'settings-search', 'Direct models')
        s.click_control('direct-models')
        click(s, 'direct-return-agent')
        click(s, 'direct-confirm')
        wait_until(lambda: binding(s, task) is None, 'return to ACP route')
        s.click_control('settings-back')
        before = event_cursor(s, task)
        s.prompt('hello')
        wait_until(lambda: prompt_finished(s, task, before), 'ACP route after Anthropic')
        assert len(requests) == 2
        s.desktop.screenshot('anthropic-and-acp-conversation', window_only=True)
        s.checks.append('anthropic-to-ACP-route-switch-preserves-conversation')
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
    result = {
        'status': 'failed',
        'checks': scenario.checks,
        'platform': 'Linux/X11/private Xvfb and owned Anthropic Messages HTTP',
    }
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
        print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
