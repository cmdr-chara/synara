import Cocoa
import CoreGraphics
import ApplicationServices

// computer-helper <command> [json-stdin]
// Commands: perms, screen-size, windows, cursor, click, move, drag, scroll,
// press, type. Output: one JSON object on stdout.

func out(_ v: Any) {
    if let d = try? JSONSerialization.data(withJSONObject: v),
       let s = String(data: d, encoding: .utf8) {
        print(s)
    }
}

func args() -> [String: Any] {
    guard CommandLine.arguments.count > 2,
          let d = CommandLine.arguments[2].data(using: .utf8),
          let o = try? JSONSerialization.jsonObject(with: d) as? [String: Any]
    else { return [:] }
    return o
}

func num(_ a: [String: Any], _ k: String) -> Double {
    (a[k] as? NSNumber)?.doubleValue ?? 0
}

func post(_ e: CGEvent?) {
    if let e { e.post(tap: .cghidEventTap) }
}

let cmd = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : ""

switch cmd {
case "perms":
    out([
        "accessibility": AXIsProcessTrusted(),
        "screenRecording": CGPreflightScreenCaptureAccess(),
    ])

case "screen-size":
    let b = CGDisplayBounds(CGMainDisplayID())
    var scale = 1.0
    if let s = NSScreen.main { scale = s.backingScaleFactor }
    out(["width": Int(b.width), "height": Int(b.height), "scale": scale])

case "windows":
    let opts: CGWindowListOption = [.optionOnScreenOnly, .excludeDesktopElements]
    let list = (CGWindowListCopyWindowInfo(opts, kCGNullWindowID) as? [[String: Any]]) ?? []
    var wins: [[String: Any]] = []
    for w in list {
        guard let pid = w[kCGWindowOwnerPID as String] as? Int,
              let num = w[kCGWindowNumber as String] as? Int,
              (w[kCGWindowLayer as String] as? Int) == 0
        else { continue }
        let b = w[kCGWindowBounds as String] as? [String: Any] ?? [:]
        wins.append([
            "id": "cua:\(pid):\(num)",
            "title": w[kCGWindowName as String] as? String ?? "",
            "appName": w[kCGWindowOwnerName as String] as? String ?? "",
            "pid": pid,
            "bounds": [
                "x": (b["X"] as? NSNumber)?.doubleValue ?? 0,
                "y": (b["Y"] as? NSNumber)?.doubleValue ?? 0,
                "width": (b["Width"] as? NSNumber)?.doubleValue ?? 0,
                "height": (b["Height"] as? NSNumber)?.doubleValue ?? 0,
            ],
            "focused": wins.isEmpty,
            "minimized": false,
            "visible": w[kCGWindowIsOnscreen as String] as? Bool ?? true,
            "stackingIndex": wins.count,
        ])
        if wins.count >= 512 { break }
    }
    out(["windows": wins])

case "cursor":
    let p = CGEvent(source: nil)?.location ?? .zero
    out(["x": p.x, "y": p.y])

case "click":
    let a = args()
    let x = num(a, "x"), y = num(a, "y")
    let right = (a["button"] as? String) == "right"
    let count = max(1, Int(num(a, "count")))
    let pt = CGPoint(x: x, y: y)
    let (dn, up, btn): (CGEventType, CGEventType, CGMouseButton) = right
        ? (.rightMouseDown, .rightMouseUp, .right)
        : (.leftMouseDown, .leftMouseUp, .left)
    for i in 0..<count {
        let d = CGEvent(mouseEventSource: nil, mouseType: dn, mouseCursorPosition: pt, mouseButton: btn)
        let u = CGEvent(mouseEventSource: nil, mouseType: up, mouseCursorPosition: pt, mouseButton: btn)
        if count > 1 {
            d?.setIntegerValueField(.mouseEventClickState, value: Int64(i + 1))
            u?.setIntegerValueField(.mouseEventClickState, value: Int64(i + 1))
        }
        post(d); post(u)
        usleep(40_000)
    }
    out(["ok": true])

case "move":
    let a = args()
    let pt = CGPoint(x: num(a, "x"), y: num(a, "y"))
    post(CGEvent(mouseEventSource: nil, mouseType: .mouseMoved, mouseCursorPosition: pt, mouseButton: .left))
    out(["ok": true])

case "drag":
    let a = args()
    let from = CGPoint(x: num(a, "fx"), y: num(a, "fy"))
    let to = CGPoint(x: num(a, "tx"), y: num(a, "ty"))
    post(CGEvent(mouseEventSource: nil, mouseType: .leftMouseDown, mouseCursorPosition: from, mouseButton: .left))
    let steps = 10
    for i in 1...steps {
        let t = Double(i) / Double(steps)
        let p = CGPoint(x: from.x + (to.x - from.x) * t, y: from.y + (to.y - from.y) * t)
        post(CGEvent(mouseEventSource: nil, mouseType: .leftMouseDragged, mouseCursorPosition: p, mouseButton: .left))
        usleep(15_000)
    }
    post(CGEvent(mouseEventSource: nil, mouseType: .leftMouseUp, mouseCursorPosition: to, mouseButton: .left))
    out(["ok": true])

case "scroll":
    let a = args()
    let e = CGEvent(scrollWheelEvent2Source: nil, units: .pixel, wheelCount: 2,
                    wheel1: Int32(num(a, "dy")), wheel2: Int32(num(a, "dx")), wheel3: 0)
    e?.location = CGPoint(x: num(a, "x"), y: num(a, "y"))
    post(e)
    out(["ok": true])

case "press":
    let a = args()
    let code = CGKeyCode(UInt16(max(0, num(a, "code"))))
    let flags = CGEventFlags(rawValue: UInt64(num(a, "flags")))
    let d = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: true)
    let u = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: false)
    d?.flags = flags; u?.flags = flags
    post(d); post(u)
    out(["ok": true])

case "type":
    let a = args()
    let text = a["text"] as? String ?? ""
    let all = Array(text.utf16)
    var i = 0
    while i < all.count {
        var units = Array(all[i..<Swift.min(i + 20, all.count)])
        let d = CGEvent(keyboardEventSource: nil, virtualKey: 0, keyDown: true)
        d?.keyboardSetUnicodeString(stringLength: units.count, unicodeString: &units)
        post(d)
        let u = CGEvent(keyboardEventSource: nil, virtualKey: 0, keyDown: false)
        post(u)
        i += 20
        usleep(5_000)
    }
    out(["ok": true])

default:
    FileHandle.standardError.write("unknown command \(cmd)\n".data(using: .utf8)!)
    exit(2)
}
