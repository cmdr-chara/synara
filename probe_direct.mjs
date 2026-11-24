import WebSocket from "ws";
const [,, port, iid, wstoken, cookie, ...calls] = process.argv;
const url = `ws://localhost:${port}/ws?x-synara-client-build=1.0.0&x-synara-protocol-epoch=1&x-synara-protocol-revision=3&x-synara-server-instance=${iid}&x-synara-uncompressed=1&token=${wstoken}`;
const ws = new WebSocket(url, { headers: { cookie } });
let id = 0;
ws.on("open", () => {
  const send = () => {
    if (!calls.length) { ws.close(); return; }
    const c = calls.shift();
    const [tag, payload] = c.split("::");
    const rid = String(id++);
    ws.send(JSON.stringify({_tag:"Request",id:rid,tag,payload:payload?JSON.parse(payload):{},headers:[]}));
    setTimeout(send, Number(process.env.GAP||1500));
  };
  send();
});
ws.on("message", (d) => console.log("<", d.toString().slice(0,220)));
ws.on("close", (c,r) => { console.log("CLOSE", c, r.toString()); process.exit(0); });
ws.on("error", (e) => console.log("ERR", e.message));
setTimeout(() => { console.log("TIMEOUT-exit"); process.exit(2); }, Number(process.env.TTL||40000));
