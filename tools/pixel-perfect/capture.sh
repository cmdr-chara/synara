#!/bin/bash
# Fresh aligned capture pair + anaglyph. Requires shell running with landing visible
# and Chrome CDP on 57012 with the web app open on the landing.
cd /tmp/pxcmp
# shell window to known position/size
osascript -e 'tell application "System Events" to tell (first process whose name contains "synara-shell") to set position of window 1 to {0, 30}' 2>/dev/null
osascript -e 'tell application "System Events" to tell (first process whose name contains "synara-shell") to set size of window 1 to {1600, 1088}' 2>/dev/null
osascript -e 'tell application "System Events" to tell (first process whose name contains "synara-shell") to set frontmost to true' 2>/dev/null
sleep 1
screencapture -R0,30,1600,1088 -x shell_landing.png
# web viewport via CDP
python3 - << 'PY'
import json, asyncio
try:
    import websockets
except ImportError:
    websockets = None
PY
python3 capture_web.py 2>/dev/null || echo "web capture skipped (capture_web.py missing)"
python3 anaglyph.py web_landing.png shell_landing.png anaglyph_landing.png
echo done
