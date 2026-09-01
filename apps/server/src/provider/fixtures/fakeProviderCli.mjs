import { spawn } from "node:child_process";

const mode = process.argv[2] ?? "success";
let child = null;

function emit(stream, payload) {
  stream.write(`${JSON.stringify(payload)}\n`);
}

function keepAlive() {
  setInterval(() => undefined, 1_000).unref();
}

function shutdown() {
  if (child && child.exitCode === null) child.kill("SIGTERM");
  process.exit(0);
}

process.once("SIGTERM", shutdown);
process.once("SIGINT", shutdown);

switch (mode) {
  case "success":
    emit(process.stdout, { event: "stdout", value: "booting" });
    emit(process.stderr, { event: "stderr", value: "diagnostic" });
    emit(process.stdout, { event: "ready" });
    keepAlive();
    break;
  case "slow":
    setTimeout(() => emit(process.stdout, { event: "ready" }), 150);
    keepAlive();
    break;
  case "never":
    emit(process.stdout, { event: "stdout", value: "started without handshake" });
    emit(process.stderr, { event: "stderr", value: "still waiting" });
    keepAlive();
    break;
  case "auth-failure":
    emit(process.stderr, { event: "authentication_failed" });
    process.exitCode = 2;
    break;
  case "crash":
    emit(process.stderr, { event: "crash" });
    process.exitCode = 3;
    break;
  case "child":
    child = spawn(process.execPath, ["-e", "setInterval(() => undefined, 1000)"], {
      stdio: "ignore",
    });
    emit(process.stdout, { event: "child", pid: child.pid });
    emit(process.stdout, { event: "ready" });
    keepAlive();
    break;
  default:
    throw new Error(`Unknown fake provider mode: ${mode}`);
}
