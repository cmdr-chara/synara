#!/usr/bin/env python3
"""Exercise native input and durable ACP behavior on a private, owned Xvfb display.

Requires Xvfb, libX11, libXtst and Pillow. Never connects to the caller's DISPLAY.
All application state, agent profiles and workspaces are created under --output.
"""
import argparse
import ctypes as C
import json
import os
from pathlib import Path
import select
import sqlite3
import subprocess
import time


def wait_until(check, label, timeout=15):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        result = check()
        if result:
            return result
        time.sleep(0.08)
    raise AssertionError(f"Timed out waiting for {label}")


class ClientMessage(C.Structure):
    _fields_ = [("type", C.c_int), ("serial", C.c_ulong), ("send_event", C.c_int),
                ("display", C.c_void_p), ("window", C.c_ulong),
                ("message_type", C.c_ulong), ("format", C.c_int),
                ("data", C.c_long * 5)]


class Event(C.Union):
    _fields_ = [("client", ClientMessage), ("padding", C.c_long * 24)]


class Desktop:
    def __init__(self, output):
        self.output = output
        self.log = (output / 'xvfb.log').open('w')
        self.xvfb = None
        self.display = None
        read, write = os.pipe()
        try:
            self.xvfb = subprocess.Popen(
                ['Xvfb', '-displayfd', str(write), '-screen', '0', '1600x1000x24',
                 '-nolisten', 'tcp', '-noreset'], pass_fds=(write,),
                stdout=self.log, stderr=subprocess.STDOUT)
            os.close(write)
            write = None
            if not select.select([read], [], [], 10)[0]:
                raise RuntimeError('Xvfb did not provide a private display')
            number = os.read(read, 32).decode().strip()
            if not number.isdecimal():
                raise RuntimeError('Invalid private display number')
            self.name = ':' + number
            self.x = C.CDLL('libX11.so.6')
            self.xt = C.CDLL('libXtst.so.6')
            self._signatures()
            self.display = self.x.XOpenDisplay(self.name.encode())
            if not self.display:
                raise RuntimeError('Could not connect to the private display')
            self.root = self.x.XDefaultRootWindow(self.display)
            self.window = None
        except BaseException:
            self.close()
            raise
        finally:
            os.close(read)
            if write is not None:
                os.close(write)

    def _signatures(self):
        def bind(lib, name, args, result=C.c_int):
            fn = getattr(lib, name)
            fn.argtypes, fn.restype = args, result
        ptr, ulong, integer = C.c_void_p, C.c_ulong, C.c_int
        bind(self.x, 'XOpenDisplay', [C.c_char_p], ptr)
        bind(self.x, 'XCloseDisplay', [ptr])
        bind(self.x, 'XDefaultRootWindow', [ptr], ulong)
        bind(self.x, 'XQueryTree', [ptr, ulong, C.POINTER(ulong), C.POINTER(ulong),
                                  C.POINTER(C.POINTER(ulong)), C.POINTER(C.c_uint)])
        bind(self.x, 'XFetchName', [ptr, ulong, C.POINTER(ptr)])
        bind(self.x, 'XFree', [ptr])
        bind(self.x, 'XGetGeometry', [ptr, ulong, C.POINTER(ulong), C.POINTER(integer),
                                    C.POINTER(integer), *([C.POINTER(C.c_uint)] * 4)])
        bind(self.x, 'XSetInputFocus', [ptr, ulong, integer, ulong])
        bind(self.x, 'XStringToKeysym', [C.c_char_p], ulong)
        bind(self.x, 'XKeysymToKeycode', [ptr, ulong], C.c_ubyte)
        bind(self.x, 'XFlush', [ptr])
        bind(self.x, 'XInternAtom', [ptr, C.c_char_p, integer], ulong)
        bind(self.x, 'XSendEvent', [ptr, ulong, integer, C.c_long, C.POINTER(Event)])
        bind(self.xt, 'XTestFakeMotionEvent', [ptr, integer, integer, integer, ulong])
        bind(self.xt, 'XTestFakeButtonEvent', [ptr, C.c_uint, integer, ulong])
        bind(self.xt, 'XTestFakeKeyEvent', [ptr, C.c_uint, integer, ulong])

    def find_window(self):
        root, parent, children, count = C.c_ulong(), C.c_ulong(), C.POINTER(C.c_ulong)(), C.c_uint()
        if not self.x.XQueryTree(self.display, self.root, C.byref(root), C.byref(parent),
                                C.byref(children), C.byref(count)):
            return None
        try:
            for window in list(children[:count.value]):
                name = C.c_void_p()
                if self.x.XFetchName(self.display, window, C.byref(name)) and name.value:
                    try:
                        if C.string_at(name) == b'Synara':
                            self.window = window
                            return window
                    finally:
                        self.x.XFree(name)
        finally:
            if children:
                self.x.XFree(children)
        return None

    def geometry(self):
        root, x, y = C.c_ulong(), C.c_int(), C.c_int()
        width, height, border, depth = (C.c_uint() for _ in range(4))
        if not self.x.XGetGeometry(self.display, self.window, C.byref(root), C.byref(x),
                                  C.byref(y), C.byref(width), C.byref(height),
                                  C.byref(border), C.byref(depth)):
            raise RuntimeError('Native window geometry unavailable')
        return x.value, y.value, width.value, height.value

    def focus(self):
        self.x.XSetInputFocus(self.display, self.window, 1, 0)
        self.x.XFlush(self.display)
        time.sleep(0.2)

    def click(self, x, y):
        left, top, width, height = self.geometry()
        if not 0 <= x < width or not 0 <= y < height:
            raise ValueError('Input must remain within the owned native window')
        self.xt.XTestFakeMotionEvent(self.display, -1, left + x, top + y, 0)
        for down in (1, 0):
            self.xt.XTestFakeButtonEvent(self.display, 1, down, 0)
        self.x.XFlush(self.display)
        time.sleep(0.25)

    def key(self, name, modifiers=()):
        def send(key, down):
            code = self.x.XKeysymToKeycode(self.display, self.x.XStringToKeysym(key.encode()))
            if not code:
                raise ValueError(f'Unmapped key {key}')
            self.xt.XTestFakeKeyEvent(self.display, code, down, 0)
        for modifier in modifiers:
            send(modifier, 1)
        send(name, 1)
        send(name, 0)
        for modifier in reversed(modifiers):
            send(modifier, 0)
        self.x.XFlush(self.display)
        time.sleep(0.035)

    def text(self, value):
        for char in value:
            self.key({' ': 'space', '-': 'minus', '\n': 'Return', '.': 'period'}.get(char, char))

    def screenshot(self, name):
        from PIL import ImageGrab
        time.sleep(0.4)
        ImageGrab.grab(xdisplay=self.name).save(self.output / (name + '.png'))

    def request_close(self):
        event = Event()
        event.client.type = 33  # ClientMessage, not a forced process termination.
        event.client.display = self.display
        event.client.window = self.window
        event.client.message_type = self.x.XInternAtom(self.display, b'WM_PROTOCOLS', 0)
        event.client.format = 32
        event.client.data[0] = self.x.XInternAtom(self.display, b'WM_DELETE_WINDOW', 0)
        event.client.data[1] = 0
        self.x.XSendEvent(self.display, self.window, 0, 0, C.byref(event))
        self.x.XFlush(self.display)
        time.sleep(0.4)

    def close(self):
        if self.display:
            self.x.XCloseDisplay(self.display)
            self.display = None
        if self.xvfb:
            stop(self.xvfb)
        self.log.close()


def stop(process):
    if process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)


class Scenario:
    def __init__(self, options):
        self.output = options.output.resolve()
        self.output.mkdir(parents=True, exist_ok=False)
        self.data = self.output / 'data'
        self.agents = self.data / 'agents'
        self.agents.mkdir(parents=True, mode=0o700)
        # An offline fixture describes a launcher that must never be executed.
        # The GUI should only persist explicit approval, without invoking npm.
        (self.agents / 'index.json').write_text(json.dumps({'version': '1.0.0', 'agents': [{
            'id': 'smoke-agent', 'name': 'Smoke Agent', 'version': '1.0.0',
            'description': 'Hermetic launcher approval fixture. Do not execute.',
            'license_url': 'https://example.com/license',
            'distribution': {'npx': {'package': '@synara-test/never-execute@1.0.0'}}
        }]}), encoding='utf-8')
        self.project = self.output / 'project'
        self.project.mkdir()
        (self.output / 'home').mkdir()
        self.runtime = self.output / 'runtime'
        self.runtime.mkdir(mode=0o700)
        self.document = self.project / 'document.txt'
        self.document.write_text('original text\n', encoding='utf-8')
        self.profiles = self.output / 'profiles.json'
        self.profiles.write_text(json.dumps([
            {'id': label, 'name': 'Fixture ' + label, 'command': str(options.fixture.resolve()),
             'args': ['--integration-fixture', label]}
            for label in ('alpha', 'beta')]), encoding='utf-8')
        self.binary = options.binary.resolve()
        self.desktop = Desktop(self.output)
        self.process = None
        self.log = None
        self.checks = []
        self.launch_count = 0

    def launch(self):
        self.log = (self.output / f'app-{len(self.checks)}.log').open('w')
        env = {key: os.environ[key] for key in ('PATH', 'LD_LIBRARY_PATH') if key in os.environ}
        env.update(DISPLAY=self.desktop.name, XDG_RUNTIME_DIR=str(self.runtime),
                   HOME=str(self.output / 'home'), GPUI_PLATFORM='x11',
                   LIBGL_ALWAYS_SOFTWARE='1', RUST_LOG='synara=info,gpui=warn')
        args = [str(self.binary), '--workspace', str(self.project), '--data-dir', str(self.data)]
        if self.launch_count == 0:
            args.extend(['--agents', str(self.profiles)])
        self.launch_count += 1
        self.process = subprocess.Popen(args, env=env, cwd=self.project,
                stdout=self.log, stderr=subprocess.STDOUT)
        def ready():
            if self.process.poll() is not None:
                raise RuntimeError('Native app exited before the window opened')
            return self.desktop.find_window()
        wait_until(ready, 'native window', 30)
        self.desktop.focus()
        assert self.desktop.geometry()[2:] == (1420, 930), 'Unexpected native layout size'
        time.sleep(0.6)

    def events(self):
        path = self.data / 'native-workspace.sqlite3'
        if not path.exists():
            return []
        with sqlite3.connect(path.as_uri() + '?mode=ro', uri=True) as db:
            return [json.loads(row[0]) for row in db.execute('SELECT data FROM events ORDER BY sequence')]

    def task(self):
        with sqlite3.connect((self.data / 'native-workspace.sqlite3').as_uri() + '?mode=ro', uri=True) as db:
            return json.loads(db.execute('SELECT data FROM tasks LIMIT 1').fetchone()[0])

    def has_text(self, text, after=0):
        return text in ''.join(e.get('text', '') for e in self.events()[after:]
                               if e['type'] == 'text_delta' and e.get('role') == 'assistant')

    def prompt(self, text):
        before = len(self.events())
        _, _, width, height = self.desktop.geometry()
        self.desktop.click(width // 2, height - 150)
        self.desktop.text(text)
        self.desktop.key('Return')
        return before

    def finished(self, after):
        wait_until(lambda: any(e['type'] == 'prompt_finished' for e in self.events()[after:]),
                   'durable prompt completion')
        wait_until(lambda: self.task()['state'] == 'completed', 'catalog completion')
        time.sleep(0.4)

    def run(self):
        self.launch()
        ui = self.desktop
        before = self.prompt('hello')
        self.finished(before)
        assert self.has_text('Hello from alpha', before)
        assert any(e['type'] == 'tool_changed' and e['patch'].get('status') == 'completed'
                   for e in self.events()[before:])
        self.checks.append('native-composer-streaming-and-tool-completion')
        ui.screenshot('conversation')

        before = self.prompt('permission')
        wait_until(lambda: self.task()['state'] == 'waiting', 'permission waiting state')
        time.sleep(0.4)
        ui.screenshot('permission')
        ui.click(418, 642)  # Deny in the fixed 1420x930 native layout.
        self.finished(before)
        assert any(e['type'] == 'permission_resolved' and e['selected'] == 'deny'
                   for e in self.events()[before:])
        self.checks.append('native-permission-denial')

        before = self.prompt('files')
        wait_until(lambda: self.task()['state'] == 'waiting', 'write permission')
        assert not (self.project / 'created.txt').exists(), 'Write must wait for approval'
        time.sleep(0.3)
        ui.click(320, 642)
        self.finished(before)
        assert (self.project / 'created.txt').read_text(encoding='utf-8') == 'native 🦀\n'
        self.checks.append('workspace-filesystem-callbacks')
        before = self.prompt('escape')
        self.finished(before)
        assert self.has_text('callback denied', before)
        self.checks.append('workspace-escape-rejected')

        # Switch profiles through the actual selector. No new backend or persisted SQL mutation.
        _, _, width, height = ui.geometry()
        ui.click(width - 170, 80)
        wait_until(lambda: self.task()['agent_id'] == 'beta', 'second fixture selected')
        before = self.prompt('hello')
        self.finished(before)
        assert self.has_text('Hello from beta', before)
        self.checks.append('second-fixture-through-same-native-controller')
        ui.screenshot('second-agent')

        events_before_approval = self.events()
        ui.key('7', ('Control_L',))
        time.sleep(0.6)
        ui.click(650, 197)
        ui.text('smoke')
        ui.screenshot('registry-search')
        ui.click(600, 335)
        ui.screenshot('registry-review')
        assert not list(self.agents.glob('agent-*/receipt.json')), 'Review is not approval'
        ui.click(400, 518)
        wait_until(lambda: list(self.agents.glob('agent-*/receipt.json')), 'explicit launcher approval')
        assert self.events() == events_before_approval, 'Installation must not start a session'
        self.checks.append('offline-registry-review-and-approval-without-execution')
        ui.screenshot('registry-approved')

        ui.key('2', ('Control_L',))
        time.sleep(0.6)
        ui.click(310, 194)  # document.txt follows created.txt in the sorted explorer.
        time.sleep(0.4)
        ui.click(850, 190)
        ui.key('a', ('Control_L',))
        ui.text('edited text\n')
        ui.request_close()
        assert self.process.poll() is None, 'Dirty close must not terminate the app'
        ui.screenshot('dirty-close')
        assert self.document.read_text() == 'original text\n'
        ui.key('Escape')
        time.sleep(0.3)
        assert self.process.poll() is None
        self.checks.append('dirty-close-cancel-preserves-document')
        ui.request_close()
        # Explicit Save and close in the centered native confirmation panel.
        ui.click(850, 540)
        wait_until(lambda: self.process.poll() is not None, 'save and close', 15)
        assert self.process.returncode == 0
        assert self.document.read_text() == 'edited text\n'
        self.checks.append('guarded-save-before-native-close')
        self.log.close()
        self.log = None
        old_events = self.events()
        self.launch()
        time.sleep(0.6)
        assert self.events() == old_events, 'Startup must not silently spawn or replay an agent'
        assert self.task()['state'] == 'completed'
        with sqlite3.connect((self.data / 'native-workspace.sqlite3').as_uri() + '?mode=ro', uri=True) as db:
            profiles = json.loads(db.execute("SELECT data FROM preferences WHERE key='agent_profiles'").fetchone()[0])
            assert any(p.get('registry') for p in profiles), 'Approved profiles must survive restart'
        ui.screenshot('restored')
        self.checks.append('durable-history-restored-without-autostart')
        ui.key('2', ('Control_L',))
        time.sleep(0.6)
        ui.click(310, 194)
        time.sleep(0.4)
        ui.click(850, 190)
        ui.key('a', ('Control_L',))
        ui.text('unsaved edit')
        self.document.write_text('external update\n', encoding='utf-8')
        ui.request_close()
        ui.click(850, 540)
        wait_until(lambda: 'File save failed. The document remains open.' in
                   Path(self.log.name).read_text(), 'visible rejected save')
        assert self.process.poll() is None
        assert self.document.read_text() == 'external update\n'
        ui.screenshot('save-conflict')
        self.checks.append('save-conflict-preserves-external-change-and-open-editor')
        # The error message adds a line to the review and shifts its buttons down.
        ui.click(700, 571)
        wait_until(lambda: self.process.poll() is not None, 'explicit discard and close', 10)
        assert self.process.returncode == 0
        assert self.document.read_text() == 'external update\n'
        self.checks.append('explicit-discard-does-not-overwrite-disk')

    def close(self):
        if self.process:
            stop(self.process)
        if self.log:
            self.log.close()
        self.desktop.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--fixture', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    options = parser.parse_args()
    scenario = Scenario(options)
    result = {'status': 'failed', 'checks': scenario.checks, 'agents': ['fixture-alpha', 'fixture-beta']}
    try:
        scenario.run()
        result['status'] = 'passed'
    except BaseException as error:
        result['error'] = str(error)
        if scenario.process and scenario.process.poll() is None:
            scenario.desktop.screenshot('failure')
        raise
    finally:
        scenario.close()
        (scenario.output / 'result.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
        print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
