import { spawn } from "node:child_process";
import { connect } from "node:net";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

type Variant = "bundled" | "slim";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const variant = process.argv[2] as Variant | undefined;

if (variant !== "bundled" && variant !== "slim") {
  throw new Error("Usage: dev.ts <bundled|slim>");
}

function spawnProcess(command: string, args: readonly string[], env = process.env) {
  return spawn(command, args, { cwd: root, env, stdio: "inherit" });
}

function viteIsRunning(): Promise<boolean> {
  return new Promise((resolvePromise) => {
    const socket = connect({ host: "127.0.0.1", port: 1420 });
    socket.once("connect", () => {
      socket.end();
      resolvePromise(true);
    });
    socket.once("error", () => {
      socket.destroy();
      resolvePromise(false);
    });
  });
}

async function waitForVite(): Promise<void> {
  const deadline = Date.now() + 30_000;
  const retry = (): Promise<void> =>
    viteIsRunning().then((running) => {
      if (running) return;
      if (Date.now() >= deadline) throw new Error("Timed out waiting for the Vite server");
      return new Promise<void>((resolvePromise) => setTimeout(resolvePromise, 100)).then(retry);
    });
  return retry();
}

const startedVite = !(await viteIsRunning());
const vite = startedVite
  ? spawnProcess("pnpm", ["--filter", "@dsh-desktop/frontend", "run", "dev"], {
      ...process.env,
      VITE_DSH_DESKTOP_BRIDGE_URL: "http://127.0.0.1:1421",
    })
  : undefined;

let desktop: ReturnType<typeof spawnProcess> | undefined;
let stopping = false;
function stop(code = 0): void {
  if (stopping) return;
  stopping = true;
  desktop?.kill();
  vite?.kill();
  process.exitCode = code;
}

vite?.once("exit", (code) => stop(code ?? 1));
process.once("SIGINT", () => stop());
process.once("SIGTERM", () => stop());

await waitForVite();
desktop = spawnProcess("pnpm", ["--filter", "@dsh-desktop/desktop", "run", `dev:${variant}`]);
desktop.once("exit", (code) => stop(code ?? 0));
