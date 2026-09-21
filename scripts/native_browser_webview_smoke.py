#!/usr/bin/env python3
"""Actual GPUI/X11 child-WebKit rendering, manual input, history and overlay test.

Uses private Xvfb, disposable data and two loopback fixture pages. Pixel evidence
is read from the real display, not OCR or a mocked browser transport.
"""
import argparse
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
            body = b'''<!doctype html><title>Real embedded WebKit</title>
<style>html,body{margin:0;background:rgb(20,150,160);height:100%;font:24px sans-serif}
button{position:absolute;left:40px;top:120px;width:240px;height:65px}</style>
<h1>Native WebKit in Synara</h1><p>This content is served by the owned HTTP fixture.</p>
<button onclick="document.body.style.background='rgb(192,94,64)';document.title='Real click confirmed'">Click native page</button>'''
            self.send_response(200)
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
    def navigate(path):
        fill(s, 'browser-address', f'http://127.0.0.1:{server.server_port}/{path}')
        s.click_control('browser-go')
        wait_until(lambda: '/'+path in requests, 'real WebKit request')
        wait_until(lambda: count((20,150,160)) > 20000, 'visible embedded web content', 20)
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
        s.click_control('browser-back')
        wait_until(lambda: requests.count('/first') >= 2, 'domain back loads prior URL')
        s.click_control('browser-forward')
        wait_until(lambda: requests.count('/second') >= 2, 'domain forward')
        s.click_control('browser-reload')
        wait_until(lambda: requests.count('/second') >= 3, 'reload')
        s.checks.append('back-forward-reload-through-existing-session-history')
        s.click_control('browser-new')
        wait_until(lambda: count((20,150,160)) == 0, 'new blank tab hides old page')
        s.click_control('browser-tab', slot=0)
        wait_until(lambda: count((20,150,160)) > 20000, 'select existing native tab')
        s.click_control('browser-close', slot=0)
        wait_until(lambda: count((20,150,160)) == 0, 'closed child destroyed')
        s.checks.append('tab-create-select-close-without-ghost-surfaces')
        s.desktop.focus()
        s.desktop.key('1', ('Control_L',))
        assert count((20,150,160)) == 0
        s.desktop.request_close()
        wait_until(lambda: s.process.poll() is not None, 'native app shutdown')
        assert s.process.returncode == 0
        s.checks.append('clean-native-shutdown')
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=3)


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
        raise
    finally:
        s.close()
        (s.output/'result.json').write_text(json.dumps(result,indent=2)+'\n')
        print(json.dumps(result,indent=2))

if __name__ == '__main__':
    main()
