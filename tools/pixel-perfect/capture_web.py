#!/usr/bin/env python3
"""Capture the web app landing at exactly 1600x1088 viewport via CDP."""
import asyncio, json, base64, urllib.request

async def main():
    # find page target on CDP 57012
    tabs = json.load(urllib.request.urlopen("http://127.0.0.1:57012/json"))
    page = next((t for t in tabs if t.get("type") == "page" and "127.0.0.1:8580" in t.get("url", "")), None)
    if not page:
        print("no synara web tab"); return
    import websockets
    async with websockets.connect(page["webSocketDebuggerUrl"], max_size=50*1024*1024) as ws:
        mid = 0
        async def cmd(method, params=None):
            nonlocal mid
            mid += 1
            await ws.send(json.dumps({"id": mid, "method": method, "params": params or {}}))
            while True:
                msg = json.loads(await ws.recv())
                if msg.get("id") == mid:
                    return msg.get("result", {})
        await cmd("Page.navigate", {"url": "http://127.0.0.1:8580/"})
        await asyncio.sleep(2)
        await cmd("Emulation.setDeviceMetricsOverride", {
            "width": 1600, "height": 1088, "deviceScaleFactor": 1, "mobile": False})
        await cmd("Page.reload")
        await asyncio.sleep(8)  # backdrop rebuild
        shot = await cmd("Page.captureScreenshot", {"format": "png"})
        with open("/tmp/pxcmp/web_landing.png", "wb") as f:
            f.write(base64.b64decode(shot["data"]))
        print("web captured")

asyncio.run(main())
