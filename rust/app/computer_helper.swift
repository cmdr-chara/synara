import Cocoa
import CoreGraphics
import ApplicationServices

// computer-helper <command> [json-stdin]
// Commands: perms, screen-size, windows, cursor, click, move, drag, scroll,
// press, type, tree, axaction. Output: one JSON object on stdout.

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

// MARK: - Accessibility tree

func axCopy(_ el: AXUIElement, _ attr: String) -> AnyObject? {
    var v: AnyObject?
    guard AXUIElementCopyAttributeValue(el, attr as CFString, &v) == .success else { return nil }
    return v
}

func axStr(_ el: AXUIElement, _ attr: String) -> String? {
    axCopy(el, attr) as? String
}

func axChildren(_ el: AXUIElement) -> [AXUIElement] {
    (axCopy(el, kAXChildrenAttribute) as? [AXUIElement]) ?? []
}

func axFrame(_ el: AXUIElement) -> CGRect {
    var pos = CGPoint.zero, size = CGSize.zero
    if let v = axCopy(el, kAXPositionAttribute) {
        var p = CGPoint.zero
        if AXValueGetValue(v as! AXValue, .cgPoint, &p) { pos = p }
    }
    if let v = axCopy(el, kAXSizeAttribute) {
        var s = CGSize.zero
        if AXValueGetValue(v as! AXValue, .cgSize, &s) { size = s }
    }
    return CGRect(origin: pos, size: size)
}

func axEditable(_ el: AXUIElement) -> Bool {
    var settable = DarwinBoolean(false)
    return AXUIElementIsAttributeSettable(el, kAXValueAttribute as CFString, &settable) == .success
        && settable.boolValue
}

/// Application element for a pid; when `wid` is given, the AX window whose
/// bounds match that CGWindow, else the focused window (fallback: first).
func axRootWindow(pid: Int, wid: Int?, bounds: CGRect?) -> AXUIElement? {
    let app = AXUIElementCreateApplication(pid_t(pid))
    let wins = (axCopy(app, kAXWindowsAttribute) as? [AXUIElement]) ?? []
    if wid != nil, let b = bounds {
        for w in wins where axFrame(w) == b { return w }
    }
    if let focused = axCopy(app, kAXFocusedWindowAttribute) { return unsafeBitCast(focused, to: AXUIElement.self) }
    return wins.first
}

/// Bounds of a CGWindow number, searched across every window (not just onscreen).
func cgWindowBounds(wid: Int) -> CGRect? {
    let list = (CGWindowListCopyWindowInfo([.optionAll, .excludeDesktopElements], kCGNullWindowID)
        as? [[String: Any]]) ?? []
    for w in list where (w[kCGWindowNumber as String] as? Int) == wid {
        let b = w[kCGWindowBounds as String] as? [String: Any] ?? [:]
        return CGRect(x: (b["X"] as? NSNumber)?.doubleValue ?? 0,
                      y: (b["Y"] as? NSNumber)?.doubleValue ?? 0,
                      width: (b["Width"] as? NSNumber)?.doubleValue ?? 0,
                      height: (b["Height"] as? NSNumber)?.doubleValue ?? 0)
    }
    return nil
}

struct AxWalk {
    var count = 0
    var truncated = false
    let maxDepth = 24
    let maxChildren = 256
    let maxNodes = 2048
}

func axLabel(_ el: AXUIElement) -> String? {
    axStr(el, kAXTitleAttribute) ?? axStr(el, kAXDescriptionAttribute)
}

func axValueString(_ el: AXUIElement) -> String? {
    let v = axCopy(el, kAXValueAttribute)
    if let s = v as? String { return s }
    if let n = v as? NSNumber { return n.stringValue }
    return nil
}

func nodeDict(_ el: AXUIElement, path: [Int], windowId: String, walk: inout AxWalk) -> [String: Any] {
    walk.count += 1
    let role = axStr(el, kAXRoleAttribute) ?? ""
    let frame = axFrame(el)
    let screen = CGDisplayBounds(CGMainDisplayID())
    var kids: [[String: Any]] = []
    var nodeTruncated = false
    if path.count < walk.maxDepth && !walk.truncated {
        let children = axChildren(el)
        let cap = Swift.min(children.count, walk.maxChildren)
        for i in 0..<cap {
            if walk.count >= walk.maxNodes { walk.truncated = true; nodeTruncated = true; break }
            kids.append(nodeDict(children[i], path: path + [i], windowId: windowId, walk: &walk))
        }
        if children.count > cap { nodeTruncated = true }
    } else if !axChildren(el).isEmpty {
        nodeTruncated = true
    }
    var d: [String: Any] = [
        "role": role,
        "label": axLabel(el) as Any,
        "value": axValueString(el) as Any,
        "description": axStr(el, kAXHelpAttribute) as Any,
        "frame": ["x": frame.origin.x, "y": frame.origin.y,
                  "width": frame.size.width, "height": frame.size.height],
        "activationPoint": frame.isEmpty ? NSNull() : [
            "x": frame.midX, "y": frame.midY,
        ],
        "onScreen": frame.intersects(screen),
        "windowId": windowId,
        "nodePath": path,
        "children": kids,
    ]
    if axEditable(el) { d["editable"] = true }
    if nodeTruncated { d["truncated"] = true }
    return d
}

/// Flat preorder list of every AXUIElement in the same walk `nodeDict` emits,
/// so a `ref` (index) resolves to the same node a `tree` listing showed.
func flatWalk(_ el: AXUIElement, path: [Int], walk: inout AxWalk,
              into out_: inout [(el: AXUIElement, path: [Int])]) {
    walk.count += 1
    out_.append((el, path))
    guard path.count < walk.maxDepth, !walk.truncated else { return }
    let children = axChildren(el)
    let cap = Swift.min(children.count, walk.maxChildren)
    for i in 0..<cap {
        if walk.count >= walk.maxNodes { walk.truncated = true; return }
        flatWalk(children[i], path: path + [i], walk: &walk, into: &out_)
    }
}

func roleMatches(_ have: String, _ want: String) -> Bool {
    func norm(_ s: String) -> String {
        var t = s.lowercased()
        if t.hasPrefix("ax") { t = String(t.dropFirst(2)) }
        return t.replacingOccurrences(of: " ", with: "")
    }
    return norm(have) == norm(want)
}

func labelMatches(_ el: AXUIElement, _ want: String) -> Bool {
    let cands = [axLabel(el), axValueString(el), axStr(el, kAXHelpAttribute)].compactMap { $0 }
    return cands.contains { $0.caseInsensitiveCompare(want) == .orderedSame }
        || cands.contains { $0.lowercased().contains(want.lowercased()) }
}

func axParseWindowId(_ s: String?) -> (pid: Int, wid: Int)? {
    guard let s else { return nil }
    let parts = s.split(separator: ":")
    guard parts.count == 3, parts[0] == "cua",
          let pid = Int(parts[1]), let wid = Int(parts[2]) else { return nil }
    return (pid, wid)
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

case "tree":
    let a = args()
    var pid = Int(num(a, "pid"))
    var wid: Int? = nil
    if let w = a["windowId"] as? String, let parsed = axParseWindowId(w) {
        pid = parsed.pid; wid = parsed.wid
    }
    if pid == 0, let front = NSWorkspace.shared.frontmostApplication {
        pid = Int(front.processIdentifier)
    }
    guard pid > 0, let root = axRootWindow(pid: pid, wid: wid,
                                         bounds: wid.flatMap(cgWindowBounds))
    else { out(["ok": false, "error": "No window could be resolved for the target."]); exit(0) }
    var walk = AxWalk()
    let windowId = a["windowId"] as? String ?? "cua:\(pid):0"
    out(["ok": true, "root": nodeDict(root, path: [], windowId: windowId, walk: &walk)])

case "axaction":
    let a = args()
    var pid = Int(num(a, "pid"))
    var wid: Int? = nil
    if let w = a["windowId"] as? String, let parsed = axParseWindowId(w) {
        pid = parsed.pid; wid = parsed.wid
    }
    guard pid > 0, let root = axRootWindow(pid: pid, wid: wid,
                                         bounds: wid.flatMap(cgWindowBounds))
    else { out(["ok": false, "error": "No window could be resolved for the target."]); exit(0) }
    var walk = AxWalk()
    var flat: [(el: AXUIElement, path: [Int])] = []
    flatWalk(root, path: [], walk: &walk, into: &flat)

    var target: AXUIElement? = nil
    if let pathArr = a["nodePath"] as? [Int] {
        if let hit = flat.first(where: { $0.path == pathArr }) {
            target = hit.el
        }
    } else {
        let wantRole = a["role"] as? String
        let wantLabel = a["label"] as? String
        let matches = flat.filter { item in
            if let r = wantRole, !roleMatches(axStr(item.el, kAXRoleAttribute) ?? "", r) {
                return false
            }
            if let l = wantLabel, !labelMatches(item.el, l) { return false }
            return wantRole != nil || wantLabel != nil
        }
        if let ref = a["ref"] as? Int {
            // ref indexes the preorder walk itself; it may also be filtered.
            if wantRole == nil && wantLabel == nil {
                if ref >= 0 && ref < flat.count { target = flat[ref].el }
            } else if ref >= 0 && ref < matches.count { target = matches[ref].el }
        } else if matches.count == 1 {
            target = matches[0].el
        } else if matches.count > 1 {
            // An ambiguous match is refused, never guessed — the caller picks
            // one with refOrdinal (or ref) just like the upstream contract.
            guard let ordinal = a["refOrdinal"] as? Int,
                  ordinal >= 0, ordinal < matches.count
            else {
                out(["ok": false, "error": "The target matched \(matches.count) elements; pick one with refOrdinal."])
                exit(0)
            }
            target = matches[ordinal].el
        }
    }
    guard let el = target else {
        out(["ok": false, "error": "No element matched the target."])
        exit(0)
    }

    let op = a["op"] as? String ?? ""
    var result: [String: Any] = ["ok": true]
    let frame = axFrame(el)
    if !frame.isEmpty {
        result["point"] = ["x": frame.midX, "y": frame.midY]
    }
    result["role"] = axStr(el, kAXRoleAttribute) ?? ""
    result["label"] = axLabel(el) as Any

    switch op {
    case "setvalue":
        guard axEditable(el) else {
            out(["ok": false, "error": "The element does not accept a value write."])
            exit(0)
        }
        let value = a["value"] as? String ?? ""
        let err = AXUIElementSetAttributeValue(el, kAXValueAttribute as CFString,
                                             value as CFTypeRef)
        if err != .success {
            out(["ok": false, "error": "AXValue write failed (\(err.rawValue))."])
            exit(0)
        }
        result["value"] = axValueString(el) as Any
    case "perform":
        var action = a["action"] as? String ?? ""
        if !action.hasPrefix("AX") {
            action = "AX" + action.prefix(1).uppercased() + action.dropFirst()
        }
        let err = AXUIElementPerformAction(el, action as CFString)
        if err != .success {
            out(["ok": false, "error": "AX action \(action) failed (\(err.rawValue))."])
            exit(0)
        }
        result["action"] = action
    case "selecttext":
        var range = CFRange(location: Int(num(a, "start")),
                            length: Int(num(a, "length")))
        guard let v = AXValueCreate(.cfRange, &range) else {
            out(["ok": false, "error": "Could not build a text range."])
            exit(0)
        }
        let err = AXUIElementSetAttributeValue(el, kAXSelectedTextRangeAttribute as CFString, v)
        if err != .success {
            out(["ok": false, "error": "AXSelectedTextRange write failed (\(err.rawValue))."])
            exit(0)
        }
    default:
        out(["ok": false, "error": "Unknown axaction op \(op)."])
        exit(0)
    }
    out(result)

default:
    FileHandle.standardError.write("unknown command \(cmd)\n".data(using: .utf8)!)
    exit(2)
}
