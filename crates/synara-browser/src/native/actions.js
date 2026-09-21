// Runs only in WebKit's named isolated world. No script-message handler or page IPC.
(function (request) {
  "use strict";
  const clip = (value, count) => String(value || "").slice(0, count);
  const visible = node => node.isConnected && !node.disabled &&
    node.getClientRects().length > 0 && getComputedStyle(node).visibility !== "hidden";
  const labelText = node => {
    const walker = document.createTreeWalker(node, NodeFilter.SHOW_TEXT);
    let value = "", visited = 0, text;
    while (value.length < 200 && visited++ < 32 && (text = walker.nextNode()))
      value += clip(text.data, 200 - value.length);
    return value;
  };
  const name = node => clip(node.getAttribute("aria-label") || labelText(node) ||
    node.getAttribute("placeholder") || node.getAttribute("name") || node.tagName, 200);
  const signature = node => JSON.stringify([node.tagName, node.type || "", name(node),
    node.getAttribute("href"), node.getAttribute("formaction")]);
  try {
    if (location.origin !== request.origin) throw new Error("Document origin changed");
    if (request.operation.operation === "read_document") {
      const refs = new Map();
      const elements = [];
      const nodes = document.createTreeWalker(document.documentElement, NodeFilter.SHOW_ELEMENT);
      let node;
      for (let i = 0; i < 4096 && elements.length < 128 && (node = nodes.nextNode()); i++) {
        if (!node.matches("a[href],button,input,textarea,select,[role=button]") ||
            !visible(node) || node.type === "file" || node.type === "hidden") continue;
        const id = "e" + request.nonce + "_" + elements.length;
        refs.set(id, {node, signature: signature(node)});
        elements.push({id, role: clip(node.getAttribute("role") || node.tagName.toLowerCase(), 64), name: name(node)});
      }
      globalThis.__synaraRefs = refs;
      // Bound traversal and UTF-8 worst-case output before it crosses the native callback.
      const walker = document.createTreeWalker(document.body || document.documentElement, NodeFilter.SHOW_TEXT);
      let text = "", visited = 0;
      while (text.length < 14000 && visited++ < 4096 && (node = walker.nextNode())) {
        if (node.parentElement && !node.parentElement.closest("script,style,noscript"))
          text += clip(node.textContent, Math.min(1000, 14000 - text.length)) + "\n";
      }
      return JSON.stringify({kind: "document", text, elements});
    }
    const op = request.operation;
    const ref = globalThis.__synaraRefs && globalThis.__synaraRefs.get(op.element);
    if (!ref) throw new Error("Element inventory is no longer available. Read the document again.");
    if (!visible(ref.node) || signature(ref.node) !== ref.signature)
      throw new Error("Element changed. Read the document again before acting.");
    const node = ref.node;
    if (op.operation === "click") {
      if (node.closest("a[href]")) throw new Error("Use an explicit navigation request for links.");
      node.click();
    } else if (op.operation === "fill") {
      if (!["INPUT", "TEXTAREA"].includes(node.tagName) || node.readOnly ||
          ["file", "hidden", "button", "submit", "reset", "image", "checkbox", "radio"].includes(node.type))
        throw new Error("This element cannot be filled");
      const prototype = node.tagName === "TEXTAREA" ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
      Object.getOwnPropertyDescriptor(prototype, "value").set.call(node, op.text);
      node.dispatchEvent(new Event("input", {bubbles: true}));
      node.dispatchEvent(new Event("change", {bubbles: true}));
    } else throw new Error("Unsupported operation");
    return JSON.stringify({kind: "done"});
  } catch (error) { return JSON.stringify({error: clip(error.message, 240)}); }
})
