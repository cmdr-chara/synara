#!/usr/bin/env python3
"""Actual GPUI/X11 child-WebKit rendering, manual input, history and overlay test.

Uses private Xvfb, disposable data and two loopback fixture pages. Pixel evidence
is read from the real display, not OCR or a mocked browser transport.
"""
import argparse
import ctypes as C
import json
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from PIL import ImageGrab
from native_smoke import Scenario, wait_until
from native_integrations_smoke import fill


def run(s):
    requests = []
    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass
        def do_GET(self):
            requests.append(self.path)
            # Make request arrival observably earlier than document commit on every run.
            time.sleep(0.6)
            body = b'''<!doctype html><title>Real embedded WebKit</title>
<style>html,body{margin:0;background:rgb(20,150,160);height:100%;font:24px sans-serif}
button{position:absolute;left:40px;top:120px;width:240px;height:65px}</style>
<h1>Native WebKit in Synara</h1><p>This content is served by the owned HTTP fixture.</p>
<button onclick="document.body.style.background='rgb(192,94,64)';document.title='Real click confirmed'">Click native page</button>'''
            if self.path == '/second':
                body = body.replace(b'rgb(20,150,160)', b'rgb(80,60,170)')
            self.send_response(200)
            self.send_header('Cache-Control', 'no-store')
            self.send_header('Content-Type', 'text/html')
            self.send_header('Content-Length', str(len(body)))
            self.end_headers()
            self.wfile.write(body)
    server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    def count(color):
        image = ImageGrab.grab(xdisplay=s.desktop.name).convert('RGB')
        return sum(n for n, c in image.getcolors(image.width * image.height) if c == color)
    def browser():
        s.click_control('command-palette')
        s.desktop.text('Browser')
        s.desktop.key('Return')
        wait_until(lambda: s.control_bounds('browser-viewport'), 'browser viewport')
    def ready(path):
        color = (80,60,170) if path == 'second' else (20,150,160)
        wait_until(lambda: s.control_bounds('browser-ready', enabled=True)
                   and count(color) > 20000, 'committed and rendered ' + path, 20)
    def navigate(path):
        fill(s, 'browser-address', f'http://127.0.0.1:{server.server_port}/{path}')
        s.click_control('browser-go')
        wait_until(lambda: '/'+path in requests, 'real WebKit request')
        ready(path)
    try:
        s.launch()
        browser()
        s.click_control('browser-new')
        navigate('first')
        s.desktop.screenshot('real-embedded-page', window_only=True)
        s.checks.append('real-child-webview-network-and-render-in-gpui')
        x,y,w,h = s.control_bounds('browser-viewport')
        s.desktop.click_client(round(x+160), round(y+152))
        wait_until(lambda: count((192,94,64)) > 20000, 'native manual DOM click')
        s.desktop.screenshot('native-page-click', window_only=True)
        s.checks.append('native-pointer-input-mutates-real-document')
        s.click_control('command-palette')
        wait_until(lambda: count((192,94,64)) == 0, 'child surface hidden behind native overlay')
        s.desktop.screenshot('native-overlay-above-webview', window_only=True)
        s.desktop.key('Escape')
        wait_until(lambda: count((192,94,64)) > 20000, 'child restored after overlay')
        navigate('second')
        s.click_control('browser-back', enabled=True, settle=0.01)
        wait_until(lambda: requests.count('/first') >= 2, 'domain back loads prior URL')
        wait_until(lambda: s.control_bounds('browser-forward', enabled=False),
                   'forward unavailable while Back is pending')
        ready('first')
        s.click_control('browser-forward', enabled=True)
        wait_until(lambda: requests.count('/second') >= 2, 'domain forward')
        ready('second')
        s.click_control('browser-reload', enabled=True)
        wait_until(lambda: requests.count('/second') >= 3, 'reload')
        ready('second')
        s.checks.append('back-forward-reload-with-delayed-native-commit')
        s.click_control('browser-new')
        wait_until(lambda: count((80,60,170)) == 0, 'new blank tab hides old page')
        s.click_control('browser-tab', slot=0)
        wait_until(lambda: count((80,60,170)) > 20000, 'select existing native tab')
        s.click_control('browser-close', slot=0)
        wait_until(lambda: count((80,60,170)) == 0, 'closed child destroyed')
        s.checks.append('tab-create-select-close-without-ghost-surfaces')
        s.desktop.focus()
        s.desktop.key('1', ('Control_L',))
        assert count((80,60,170)) == 0
        s.desktop.request_close()
        wait_until(lambda: s.process.poll() is not None, 'native app shutdown')
        assert s.process.returncode == 0
        s.checks.append('clean-native-shutdown')
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=3)


def native_tree(s):
    """Bounded failure diagnostics on this test's private X server only."""
    class Attributes(C.Structure):
        _fields_ = [(name, C.c_int) for name in ('x','y','width','height','border','depth')] + [
            ('visual', C.c_void_p), ('root', C.c_ulong), ('klass', C.c_int),
            ('bit_gravity', C.c_int), ('win_gravity', C.c_int), ('backing_store', C.c_int),
            ('backing_planes', C.c_ulong), ('backing_pixel', C.c_ulong), ('save_under', C.c_int),
            ('colormap', C.c_ulong), ('map_installed', C.c_int), ('map_state', C.c_int),
            ('all_events', C.c_long), ('your_events', C.c_long), ('no_propagate', C.c_long),
            ('override_redirect', C.c_int), ('screen', C.c_void_p)]
    x = s.desktop.x
    x.XGetWindowAttributes.argtypes = [C.c_void_p, C.c_ulong, C.POINTER(Attributes)]
    x.XGetWindowAttributes.restype = C.c_int
    queue = [(s.desktop.window, 0)]
    result = []
    while queue and len(result) < 64:
        window, level = queue.pop(0)
        attrs = Attributes()
        if not x.XGetWindowAttributes(s.desktop.display, window, C.byref(attrs)):
            continue
        result.append(dict(id=window, level=level, x=attrs.x, y=attrs.y,
            width=attrs.width, height=attrs.height, map_state=attrs.map_state))
        if level >= 5:
            continue
        root, parent, count = C.c_ulong(), C.c_ulong(), C.c_uint()
        children = C.POINTER(C.c_ulong)()
        if x.XQueryTree(s.desktop.display, window, C.byref(root), C.byref(parent), C.byref(children), C.byref(count)):
            try:
                queue.extend((child, level+1) for child in children[:min(count.value, 64)])
            finally:
                if children:
                    x.XFree(children)
    (s.output/'native-window-tree.json').write_text(json.dumps(result, indent=2)+'\n')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--fixture', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    s = Scenario(parser.parse_args())
    result = {'status':'failed', 'checks':s.checks, 'platform':'Linux/X11/private Xvfb'}
    try:
        run(s)
        result['status'] = 'passed'
    except BaseException as error:
        result['error'] = str(error)
        if s.process and s.process.poll() is None:
            s.desktop.screenshot('failure', window_only=True)
            native_tree(s)
        raise
    finally:
        s.close()
        (s.output/'result.json').write_text(json.dumps(result,indent=2)+'\n')
        print(json.dumps(result,indent=2))

if __name__ == '__main__':
    main()
