import { spawn } from "node:child_process";
import { cp, mkdir, rm } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

type DistributionVariant = "bundled" | "slim";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");

function run(command: string, args: readonly string[], env = process.env): Promise<void> {
  return new Promise((resolvePromise, reject) => {
    const child = spawn(command, args, { cwd: root, env, stdio: "inherit" });
    child.once("error", reject);
    child.once("exit", (code) =>
      code === 0
        ? resolvePromise()
        : reject(new Error(`${command} exited with code ${String(code)}`)),
    );
  });
}

function platform(): string {
  if (process.platform === "win32") return "windows";
  if (process.platform === "darwin") return "macos";
  return "linux";
}

function architecture(): string {
  if (process.arch === "x64") return "x86_64";
  if (process.arch === "arm64") return "aarch64";
  throw new Error(`Unsupported architecture: ${process.arch}`);
}

async function main(): Promise<void> {
  const variant = process.argv[2] as DistributionVariant | undefined;
  if (variant !== "bundled" && variant !== "slim") {
    throw new Error("Usage: build.ts <bundled|slim>");
  }
  if (variant === "bundled") await run("pnpm", ["run", "runtime:prepare"]);
  await run("pnpm", ["run", "frontend:build"]);
  const env = {
    ...process.env,
    CARGO_TARGET_DIR: join(root, "apps/desktop/target"),
    DSH_DESKTOP_VARIANT: variant,
  };
  await run("cargo", ["build", "--release", "--manifest-path", "apps/desktop/Cargo.toml"], env);

  const executable = join(
    root,
    "apps/desktop/target/release",
    process.platform === "win32" ? "dsh-desktop.exe" : "dsh-desktop",
  );
  const staging = join(root, "apps/desktop/target/release/bundle", `dsh-desktop-${variant}`);
  const installers = join(root, "apps/desktop/target/release/bundle/installers");
  await rm(staging, { recursive: true, force: true });
  await mkdir(staging, { recursive: true });
  await cp(
    executable,
    join(staging, process.platform === "win32" ? "dsh-desktop.exe" : "dsh-desktop"),
  );
  await cp(join(root, "apps/frontend/dist"), join(staging, "frontend"), { recursive: true });
  await cp(join(root, "apps/desktop/icons"), join(staging, "icons"), { recursive: true });
  if (variant === "bundled") {
    await cp(join(root, "apps/desktop/runtime/bundled"), join(staging, "runtime/bundled"), {
      recursive: true,
    });
  }
  await mkdir(installers, { recursive: true });
  const archive = `dsh-desktop-${process.env.npm_package_version ?? "0.1.1"}-${variant}-${platform()}-${architecture()}.tar.gz`;
  await run("tar", [
    "-czf",
    join(installers, archive),
    "-C",
    dirname(staging),
    `dsh-desktop-${variant}`,
  ]);
}

await main();
