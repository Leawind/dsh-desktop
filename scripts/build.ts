import { spawn } from "node:child_process";
import { copyFile, mkdir, readFile, rm } from "node:fs/promises";
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
  const artifacts = join(root, "apps/desktop/target/release/artifacts");
  await mkdir(artifacts, { recursive: true });
  const version = (await readFile(join(root, "apps/desktop/Cargo.toml"), "utf8")).match(
    /^version\s*=\s*"([^"]+)"$/mu,
  )?.[1];
  if (!version) throw new Error("Could not read the desktop package version");
  const filename = `dsh-desktop-${version}-${variant}-${platform()}-${architecture()}${
    process.platform === "win32" ? ".exe" : ""
  }`;
  await rm(join(artifacts, filename), { force: true });
  await copyFile(executable, join(artifacts, filename));
}

await main();
