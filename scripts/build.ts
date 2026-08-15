import { createHash } from "node:crypto";
import { mkdir, readFile, readdir, rename, rm } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawn } from "node:child_process";

type DistributionVariant = "bundled" | "slim";
type AppImageArchitecture = "x86_64" | "aarch64";

interface AppImageRuntimeDefinition {
  url: string;
  sha256: string;
}

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const definitionsPath = join(repositoryRoot, "scripts", "appimage-runtimes.json");

function argument(name: string): string | undefined {
  const index = process.argv.indexOf(`--${name}`);
  return index >= 0 ? process.argv[index + 1] : undefined;
}

async function run(
  command: string,
  args: readonly string[],
  environment: NodeJS.ProcessEnv = process.env,
): Promise<void> {
  await new Promise<void>((resolvePromise, reject) => {
    const child = spawn(command, args, {
      cwd: repositoryRoot,
      env: environment,
      stdio: "inherit",
    });
    child.once("error", reject);
    child.once("exit", (code, signal) => {
      if (code === 0) resolvePromise();
      else
        reject(
          new Error(
            `${command} exited with ${signal ? `signal ${signal}` : `code ${String(code)}`}`,
          ),
        );
    });
  });
}

async function capture(command: string, args: readonly string[]): Promise<string> {
  return await new Promise<string>((resolvePromise, reject) => {
    const child = spawn(command, args, {
      cwd: repositoryRoot,
      stdio: ["ignore", "pipe", "inherit"],
    });
    let output = "";
    child.stdout.setEncoding("utf8");
    child.stdout.on("data", (chunk: string) => (output += chunk));
    child.once("error", reject);
    child.once("exit", (code) => {
      if (code === 0) resolvePromise(output);
      else reject(new Error(`${command} exited with code ${String(code)}`));
    });
  });
}

async function sha256(path: string): Promise<string> {
  return createHash("sha256")
    .update(await readFile(path))
    .digest("hex");
}

function appImageArchitecture(): AppImageArchitecture {
  if (process.arch === "x64") return "x86_64";
  if (process.arch === "arm64") return "aarch64";
  throw new Error(`AppImage packaging is not configured for ${process.arch}`);
}

async function verify(path: string, expectedSha256: string): Promise<boolean> {
  try {
    return (await sha256(path)) === expectedSha256;
  } catch {
    return false;
  }
}

async function prepareAppImageRuntime(): Promise<string | undefined> {
  if (process.platform !== "linux") return undefined;

  const architecture = appImageArchitecture();
  const definitions = JSON.parse(await readFile(definitionsPath, "utf8")) as Record<
    AppImageArchitecture,
    AppImageRuntimeDefinition
  >;
  const definition = definitions[architecture];
  const manuallyProvided = argument("appimage-runtime");
  if (manuallyProvided) {
    const path = resolve(manuallyProvided);
    if (!(await verify(path, definition.sha256))) {
      throw new Error(`AppImage runtime checksum mismatch: ${path}`);
    }
    return path;
  }

  const cached = join(
    repositoryRoot,
    ".cache",
    "build-tools",
    "appimage",
    `runtime-${architecture}`,
  );
  if (await verify(cached, definition.sha256)) {
    console.log(`Using cached AppImage runtime at ${cached}`);
    return cached;
  }

  await mkdir(dirname(cached), { recursive: true });
  await rm(cached, { force: true });
  const temporary = `${cached}.download-${String(process.pid)}`;
  try {
    await run("curl", [
      "--fail",
      "--location",
      "--retry",
      "3",
      "--output",
      temporary,
      definition.url,
    ]);
    if (!(await verify(temporary, definition.sha256))) {
      throw new Error(
        `Downloaded AppImage runtime checksum mismatch; update ${definitionsPath} deliberately if the upstream runtime changed`,
      );
    }
    await rename(temporary, cached);
  } finally {
    await rm(temporary, { force: true });
  }
  return cached;
}

async function cleanLinuxBundleStaging(): Promise<void> {
  if (process.platform !== "linux") return;

  const metadata = JSON.parse(
    await capture("cargo", [
      "metadata",
      "--manifest-path",
      "apps/desktop/Cargo.toml",
      "--format-version",
      "1",
      "--no-deps",
    ]),
  ) as { target_directory: string };
  const bundleDirectory = join(metadata.target_directory, "release", "bundle");
  await rm(join(bundleDirectory, "appimage_deb"), { recursive: true, force: true });
  await Promise.all(
    ["appimage", "deb", "rpm"].map(async (format) => {
      const directory = join(bundleDirectory, format);
      let entries;
      try {
        entries = await readdir(directory, { withFileTypes: true });
      } catch (error) {
        if ((error as NodeJS.ErrnoException).code === "ENOENT") return;
        throw error;
      }
      await Promise.all(
        entries
          .filter((entry) => entry.isDirectory())
          .map((entry) => rm(join(directory, entry.name), { recursive: true, force: true })),
      );
    }),
  );
}

async function main(): Promise<void> {
  const variant = process.argv[2] as DistributionVariant | undefined;
  if (variant !== "bundled" && variant !== "slim") {
    throw new Error(
      "Usage: build.ts <bundled|slim> [--bundles <targets>] [--appimage-runtime <path>]",
    );
  }

  if (variant === "bundled") await run("pnpm", ["run", "runtime:prepare"]);
  const appImageRuntime = await prepareAppImageRuntime();
  await cleanLinuxBundleStaging();
  const environment = {
    ...process.env,
    DSH_DESKTOP_VARIANT: variant,
    ...(appImageRuntime ? { LDAI_RUNTIME_FILE: appImageRuntime } : {}),
    ...(process.platform === "linux" ? { NO_STRIP: "1" } : {}),
  };
  const tauriArguments = ["build", "--config", `tauri.${variant}.conf.json`];
  const bundles = argument("bundles");
  if (bundles) tauriArguments.push("--bundles", bundles);
  await run(
    "pnpm",
    ["--filter", "@dsh-desktop/desktop", "exec", "tauri", ...tauriArguments],
    environment,
  );
  await run("pnpm", ["run", "bundle:label", variant], environment);
}

await main();
