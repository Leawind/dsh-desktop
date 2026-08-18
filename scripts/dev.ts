import { type ChildProcess, spawn } from "node:child_process";
import { connect } from "node:net";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

type Variant = "bundled" | "slim";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const frontendDirectory = resolve(root, "apps/frontend");
const desktopDirectory = resolve(root, "apps/desktop");
const variant = process.argv[2] as Variant | undefined;

if (variant !== "bundled" && variant !== "slim") {
  throw new Error("Usage: dev.ts <bundled|slim>");
}

function spawnProcess(command: string, args: readonly string[], env = process.env, cwd = root) {
  return spawn(command, args, { cwd, env, stdio: ["ignore", "inherit", "inherit"] });
}

function exited(process: ChildProcess): Promise<number> {
  return new Promise((resolvePromise) => {
    process.once("exit", (code) => resolvePromise(code ?? 0));
    process.once("error", () => resolvePromise(1));
  });
}

function terminate(process: ChildProcess | undefined): Promise<number> {
  if (!process || process.exitCode !== null) return Promise.resolve(process?.exitCode ?? 0);
  const result = exited(process);
  process.kill();
  return result;
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
  ? spawnProcess(
      resolve(frontendDirectory, "node_modules/.bin/vite"),
      [],
      { ...process.env, VITE_DSH_DESKTOP_BRIDGE_URL: "http://127.0.0.1:1421" },
      frontendDirectory,
    )
  : undefined;

let desktop: ChildProcess | undefined;
let stopping = false;
async function stop(code = 0): Promise<void> {
  if (stopping) return;
  stopping = true;
  await Promise.all([terminate(desktop), terminate(vite)]);
  process.exitCode = code;
}

vite?.once("exit", (code) => void stop(code ?? 1));
process.once("SIGINT", () => void stop());
process.once("SIGTERM", () => void stop());

await waitForVite();
desktop = spawnProcess(
  "cargo",
  ["run", "--manifest-path", "Cargo.toml"],
  {
    ...process.env,
    DSH_DESKTOP_DEVELOPMENT: "1",
    DSH_DESKTOP_VARIANT: variant,
    DSH_DESKTOP_FRONTEND_URL: "http://127.0.0.1:1420",
    DSH_DESKTOP_BRIDGE_PORT: "1421",
  },
  desktopDirectory,
);
await stop(await exited(desktop));
