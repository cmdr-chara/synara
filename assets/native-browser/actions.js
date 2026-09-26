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
    if (request.operation.operation === "web_mcp_tools") {
      if (location.protocol !== "https:" &&
          !(location.protocol === "http:" && ["localhost", "127.0.0.1", "::1"].includes(location.hostname)))
        throw new Error("WebMCP declarations require a secure context.");
      const refs = new Map(), tools = [];
      const forms = Array.from(document.querySelectorAll("form[toolname][tooldescription]")).slice(0, 32);
      for (const form of forms) {
        const toolName = clip(form.getAttribute("toolname"), 128).trim();
        const description = clip(form.getAttribute("tooldescription"), 2048).trim();
        if (!/^[A-Za-z0-9_.-]{1,128}$/.test(toolName) || !description) continue;
        const properties = {}, required = [], controls = Array.from(form.elements).slice(0, 128);
        let unsupported = false;
        for (const control of controls) {
          if (!control.name || control.disabled || control.form !== form ||
              !["INPUT", "TEXTAREA", "SELECT"].includes(control.tagName)) continue;
          const field = clip(control.name, 128), type = String(control.type || "").toLowerCase();
          if (!/^[A-Za-z0-9_.-]{1,128}$/.test(field) || properties[field] ||
              ["file", "password", "hidden"].includes(type)) { unsupported = true; break; }
          const schema = {type: type === "checkbox" ? "boolean" :
            ["number", "range"].includes(type) ? "number" : "string"};
          const fieldDescription = clip(control.getAttribute("toolparamdescription"), 1024).trim();
          if (fieldDescription) schema.description = fieldDescription;
          if (control.tagName === "SELECT") {
            const values = Array.from(control.options).filter(option => !option.disabled)
              .slice(0, 64).map(option => clip(option.value, 512));
            if (values.length) schema.enum = values;
          }
          properties[field] = schema;
          if (control.required) required.push(field);
        }
        if (unsupported) continue;
        const schema = {type: "object", properties, additionalProperties: false};
        if (required.length) schema.required = required;
        const id = "w" + request.nonce + "_" + tools.length;
        const signatureValue = JSON.stringify([
          toolName, description, form.hasAttribute("toolautosubmit"),
          form.getAttribute("action"), form.getAttribute("method"),
          controls.map(control => [control.tagName, control.type || "", control.name || "",
            control.required, control.getAttribute("toolparamdescription")])
        ]);
        refs.set(id, {form, signature: signatureValue});
        tools.push({id, name: toolName, description,
          auto_submit: form.hasAttribute("toolautosubmit"),
          input_schema: JSON.stringify(schema)});
      }
      globalThis.__synaraWebMcpRefs = refs;
      return JSON.stringify({kind: "web_mcp_tools", tools});
    }
    if (request.operation.operation === "read_document") {
      const refs = new Map();
      const elements = [];
      const nodes = document.createTreeWalker(document.documentElement, NodeFilter.SHOW_ELEMENT);
      let node;
      for (let i = 0; i < 4096 && elements.length < 128 && (node = nodes.nextNode()); i++) {
        if (!node.matches("a[href],button,input,textarea,select,[role=button]") ||
            !visible(node) || node.type === "hidden") continue;
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
    if (op.operation === "web_mcp_invoke") {
      const ref = globalThis.__synaraWebMcpRefs && globalThis.__synaraWebMcpRefs.get(op.tool_id);
      if (!ref || !ref.form.isConnected)
        throw new Error("WebMCP tool inventory changed. Discover tools again.");
      const form = ref.form, controls = Array.from(form.elements).slice(0, 128);
      const signatureValue = JSON.stringify([
        clip(form.getAttribute("toolname"), 128).trim(),
        clip(form.getAttribute("tooldescription"), 2048).trim(),
        form.hasAttribute("toolautosubmit"),
        form.getAttribute("action"), form.getAttribute("method"),
        controls.map(control => [control.tagName, control.type || "", control.name || "",
          control.required, control.getAttribute("toolparamdescription")])
      ]);
      if (signatureValue !== ref.signature)
        throw new Error("WebMCP tool changed. Discover tools again.");
      const args = op.arguments && typeof op.arguments === "object" && !Array.isArray(op.arguments)
        ? op.arguments : {};
      const known = new Set();
      for (const control of controls) {
        if (!control.name || control.disabled || control.form !== form ||
            !["INPUT", "TEXTAREA", "SELECT"].includes(control.tagName)) continue;
        const field = control.name, type = String(control.type || "").toLowerCase();
        known.add(field);
        if (!Object.prototype.hasOwnProperty.call(args, field)) {
          if (control.required) throw new Error("Missing required WebMCP field: " + field);
          continue;
        }
        const value = args[field];
        if (type === "checkbox") {
          if (typeof value !== "boolean") throw new Error("WebMCP checkbox requires boolean.");
          control.checked = value;
        } else if (type === "radio") {
          const selected = controls.find(candidate =>
            candidate.name === field && candidate.type === "radio" &&
            String(candidate.value) === String(value));
          if (!selected) throw new Error("WebMCP radio value is unavailable.");
          selected.checked = true;
        } else if (control.tagName === "SELECT") {
          const values = Array.isArray(value) ? value.map(String) : [String(value)];
          for (const option of control.options) option.selected = values.includes(option.value);
          if (!Array.from(control.selectedOptions).length)
            throw new Error("WebMCP select value is unavailable.");
        } else {
          if (value !== null && !["string", "number", "boolean"].includes(typeof value))
            throw new Error("WebMCP field must be a scalar value.");
          const prototype = control.tagName === "TEXTAREA"
            ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
          Object.getOwnPropertyDescriptor(prototype, "value").set.call(
            control, value === null ? "" : String(value));
        }
        control.dispatchEvent(new Event("input", {bubbles: true}));
        control.dispatchEvent(new Event("change", {bubbles: true}));
      }
      for (const key of Object.keys(args))
        if (!known.has(key)) throw new Error("Unknown WebMCP field: " + key);
      if (form.hasAttribute("toolautosubmit")) {
        form.requestSubmit();
      } else {
        const submitter = form.querySelector('button[type="submit"],input[type="submit"],button:not([type])');
        if (submitter) {
          submitter.focus({preventScroll: true});
          submitter.scrollIntoView({block: "center", inline: "nearest"});
        } else {
          form.scrollIntoView({block: "center", inline: "nearest"});
        }
      }
      return JSON.stringify({kind: "done"});
    }
    if (op.operation === "input") {
      const scroll = op.event && op.event.scroll;
      if (!scroll || !Number.isInteger(scroll.x) || !Number.isInteger(scroll.y) ||
          Math.abs(scroll.x) > 4096 || Math.abs(scroll.y) > 4096)
        throw new Error("Only bounded page scroll input is supported");
      // A fresh document read is required after viewport changes. Never reuse
      // pre-scroll references, including operations queued before this action.
      globalThis.__synaraRefs = new Map();
      window.scrollBy({left: scroll.x, top: scroll.y, behavior: "instant"});
      return JSON.stringify({kind: "done"});
    }
    const refId = op.element || op.download_id || op.chooser_id;
    const ref = globalThis.__synaraRefs && globalThis.__synaraRefs.get(refId);
    if (!ref) throw new Error("Element inventory is no longer available. Read the document again.");
    if (!visible(ref.node) || signature(ref.node) !== ref.signature)
      throw new Error("Element changed. Read the document again before acting.");
    const node = ref.node;
    if (op.operation === "click") {
      if (node.closest("a[href]")) throw new Error("Use an explicit navigation request for links.");
      node.click();
    } else if (op.operation === "download") {
      const link = node.closest("a[href]");
      if (!link) throw new Error("Download target is not a link.");
      const target = new URL(link.href, location.href);
      if (!["http:", "https:"].includes(target.protocol) || target.origin !== location.origin)
        throw new Error("Downloads must stay on the committed origin.");
      return JSON.stringify({kind: "download_target", url: target.toString()});
    } else if (op.operation === "upload") {
      if (node.tagName !== "INPUT" || node.type !== "file" || node.disabled)
        throw new Error("Upload target is not an enabled file input.");
      node.click();
      return JSON.stringify({kind: "upload_requested"});
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
