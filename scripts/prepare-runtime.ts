import { createHash } from "node:crypto";
import { createReadStream, createWriteStream } from "node:fs";
import {
  chmod,
  cp,
  mkdir,
  mkdtemp,
  readFile,
  readdir,
  rename,
  rm,
  stat,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawn } from "node:child_process";
import { pipeline } from "node:stream/promises";
import { constants as zlibConstants, createGzip } from "node:zlib";

type Platform = "linux" | "darwin" | "win32";
type Architecture = "x64" | "arm64";

interface RuntimeVersions {
  node: string;
  dsh: string;
  pnpm: string;
}

interface RuntimeTarget {
  platform: Platform;
  architecture: Architecture;
  nodePlatform: string;
  archiveExtension: ".tar.xz" | ".zip";
  triple: string;
}

interface RuntimeFile {
  path: string;
  sha256: string;
}

interface RuntimeManifest {
  schemaVersion: number;
  runtimeId: string;
  target: string;
  nodeVersion: string;
  dshVersion: string;
  pnpmVersion: string;
  definitionSha256: string;
  archive: RuntimeFile;
  files: RuntimeFile[];
}

interface PackageNotice {
  name: string;
  version: string;
  license: string;
}

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const scriptPath = fileURLToPath(import.meta.url);
const versionsPath = join(repositoryRoot, "runtime", "versions.json");
const npmRegistry = "https://registry.npmjs.org";

function argument(name: string): string | undefined {
  const index = process.argv.indexOf(`--${name}`);
  return index >= 0 ? process.argv[index + 1] : undefined;
}

function targetFromArguments(): RuntimeTarget {
  const platform = (argument("platform") ?? process.platform) as Platform;
  const architecture = (argument("arch") ?? process.arch) as Architecture;
  const supported = platform === "linux" || platform === "darwin" || platform === "win32";
  if (!supported || (architecture !== "x64" && architecture !== "arm64")) {
    throw new Error(`Unsupported runtime target: ${platform}-${architecture}`);
  }

  const nodePlatform = platform === "win32" ? "win" : platform;
  const archiveExtension = platform === "win32" ? ".zip" : ".tar.xz";
  const triplePlatform =
    platform === "linux"
      ? "unknown-linux-gnu"
      : platform === "darwin"
        ? "apple-darwin"
        : "pc-windows-msvc";
  const tripleArchitecture = architecture === "x64" ? "x86_64" : "aarch64";
  return {
    platform,
    architecture,
    nodePlatform,
    archiveExtension,
    triple: `${tripleArchitecture}-${triplePlatform}`,
  };
}

async function run(command: string, args: readonly string[], cwd?: string): Promise<void> {
  await new Promise<void>((resolvePromise, reject) => {
    const child = spawn(command, args, { cwd, stdio: "inherit" });
    child.once("error", reject);
    child.once("exit", (code) => {
      if (code === 0) resolvePromise();
      else reject(new Error(`${command} exited with code ${String(code)}`));
    });
  });
}

async function capture(command: string, args: readonly string[], cwd?: string): Promise<string> {
  return await new Promise<string>((resolvePromise, reject) => {
    const child = spawn(command, args, { cwd, stdio: ["ignore", "pipe", "inherit"] });
    let output = "";
    child.stdout.setEncoding("utf8");
    child.stdout.on("data", (chunk: string) => (output += chunk));
    child.once("error", reject);
    child.once("exit", (code) => {
      if (code === 0) resolvePromise(output.trim());
      else reject(new Error(`${command} exited with code ${String(code)}`));
    });
  });
}

async function download(url: string, output: string): Promise<void> {
  await mkdir(dirname(output), { recursive: true });
  try {
    await stat(output);
    return;
  } catch {
    // The cache entry does not exist yet.
  }
  await run("curl", ["--fail", "--location", "--retry", "3", "--output", output, url]);
}

async function sha256(path: string): Promise<string> {
  return createHash("sha256")
    .update(await readFile(path))
    .digest("hex");
}

async function definitionSha256(): Promise<string> {
  const digest = createHash("sha256");
  const paths = [
    scriptPath,
    versionsPath,
    join(repositoryRoot, "runtime", "package.json"),
    join(repositoryRoot, "runtime", "package-lock.json"),
  ];
  const contents = await Promise.all(paths.map((path) => readFile(path)));
  for (const content of contents) digest.update(content);
  return digest.digest("hex");
}

async function runtimeIsCurrent(
  output: string,
  target: RuntimeTarget,
  versions: RuntimeVersions,
  definition: string,
): Promise<boolean> {
  try {
    const manifest = JSON.parse(
      await readFile(join(output, "manifest.json"), "utf8"),
    ) as RuntimeManifest;
    if (
      manifest.schemaVersion !== 1 ||
      manifest.target !== target.triple ||
      manifest.nodeVersion !== versions.node ||
      manifest.dshVersion !== versions.dsh ||
      manifest.pnpmVersion !== versions.pnpm ||
      manifest.definitionSha256 !== definition
    ) {
      return false;
    }
    return (await sha256(join(output, manifest.archive.path))) === manifest.archive.sha256;
  } catch {
    return false;
  }
}

async function verifyNodeArchive(
  archive: string,
  archiveName: string,
  checksums: string,
): Promise<void> {
  const checksumLine = (await readFile(checksums, "utf8"))
    .split(/\r?\n/u)
    .find((line) => line.endsWith(`  ${archiveName}`));
  if (!checksumLine) throw new Error(`Node.js checksum is missing for ${archiveName}`);
  const expected = checksumLine.split(/\s+/u)[0];
  const actual = await sha256(archive);
  if (actual !== expected) throw new Error(`Node.js archive checksum mismatch for ${archiveName}`);
}

async function findExtractedRoot(directory: string): Promise<string> {
  const entries = await readdir(directory, { withFileTypes: true });
  const roots = entries.filter((entry) => entry.isDirectory());
  if (roots.length !== 1 || !roots[0]) throw new Error("Unexpected Node.js archive layout");
  return join(directory, roots[0].name);
}

async function collectPackageNotices(root: string): Promise<PackageNotice[]> {
  const notices = new Map<string, PackageNotice>();
  const entries = await readdir(root, { recursive: true, withFileTypes: true });
  const packageNotices = await Promise.all(
    entries
      .filter((entry) => entry.isFile() && entry.name === "package.json")
      .map(async (entry): Promise<PackageNotice | undefined> => {
        const path = join(entry.parentPath, entry.name);
        const value = JSON.parse(await readFile(path, "utf8")) as {
          name?: string;
          version?: string;
          license?: string | { type?: string };
        };
        if (!value.name || !value.version) return undefined;
        const license =
          typeof value.license === "string" ? value.license : (value.license?.type ?? "UNKNOWN");
        return {
          name: value.name,
          version: value.version,
          license,
        };
      }),
  );
  for (const notice of packageNotices) {
    if (notice) notices.set(`${notice.name}@${notice.version}`, notice);
  }
  return [...notices.values()].toSorted((left, right) =>
    `${left.name}@${left.version}`.localeCompare(`${right.name}@${right.version}`),
  );
}

async function pruneIncompatibleLinuxNativeArtifacts(
  appDirectory: string,
  target: RuntimeTarget,
): Promise<void> {
  if (target.platform !== "linux") return;

  const koffiPackages = join(appDirectory, "node_modules", "@koromix");
  let packages;
  try {
    packages = await readdir(koffiPackages, { withFileTypes: true });
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return;
    throw error;
  }
  await Promise.all(
    packages
      .filter(
        (packageEntry) =>
          packageEntry.isDirectory() && packageEntry.name.startsWith("koffi-linux-"),
      )
      .map(async (packageEntry) => {
        const packageDirectory = join(koffiPackages, packageEntry.name);
        const binaryDirectories = await readdir(packageDirectory, { withFileTypes: true });
        await Promise.all(
          binaryDirectories
            .filter((entry) => entry.isDirectory() && entry.name.startsWith("musl_"))
            .map((entry) =>
              rm(join(packageDirectory, entry.name), { recursive: true, force: true }),
            ),
        );
      }),
  );
}

function portablePath(root: string, path: string): string {
  return relative(root, path).split("\\").join("/");
}

async function main(): Promise<void> {
  const versions = JSON.parse(await readFile(versionsPath, "utf8")) as RuntimeVersions;
  const target = targetFromArguments();
  const output = resolve(
    argument("output") ?? join(repositoryRoot, "apps", "desktop", "runtime", "bundled"),
  );
  const cache = resolve(argument("cache") ?? join(repositoryRoot, ".cache", "runtime"));
  const definition = await definitionSha256();
  if (!argument("force") && (await runtimeIsCurrent(output, target, versions, definition))) {
    console.log(`Bundled runtime is already current at ${output}`);
    return;
  }
  const archiveName = `node-v${versions.node}-${target.nodePlatform}-${target.architecture}${target.archiveExtension}`;
  const archive = resolve(argument("node-archive") ?? join(cache, archiveName));
  const checksums = join(cache, `node-v${versions.node}-SHASUMS256.txt`);
  const baseUrl = `https://nodejs.org/dist/v${versions.node}`;

  if (!argument("node-archive")) await download(`${baseUrl}/${archiveName}`, archive);
  await download(`${baseUrl}/SHASUMS256.txt`, checksums);
  await verifyNodeArchive(archive, archiveName, checksums);

  const temporary = await mkdtemp(join(tmpdir(), "dsh-desktop-runtime-"));
  const extraction = join(temporary, "node-extracted");
  const assembled = join(temporary, "bundled");
  const payload = join(assembled, "payload");
  const nodeDirectory = join(payload, "node");
  const appDirectory = join(payload, "app");
  await mkdir(extraction, { recursive: true });
  await run("tar", ["-xf", archive, "-C", extraction]);
  await mkdir(payload, { recursive: true });
  await rename(await findExtractedRoot(extraction), nodeDirectory);
  await mkdir(appDirectory, { recursive: true });

  const runtimePackageDirectory = join(repositoryRoot, "runtime");
  const runtimePackage = JSON.parse(
    await readFile(join(runtimePackageDirectory, "package.json"), "utf8"),
  ) as { dependencies?: Record<string, string> };
  if (
    runtimePackage.dependencies?.["@deepseek-ai/dsh"] !== versions.dsh ||
    runtimePackage.dependencies?.pnpm !== versions.pnpm
  ) {
    throw new Error("runtime/package.json does not match runtime/versions.json");
  }
  await cp(join(runtimePackageDirectory, "package.json"), join(appDirectory, "package.json"));
  await cp(
    join(runtimePackageDirectory, "package-lock.json"),
    join(appDirectory, "package-lock.json"),
  );

  const nodeExecutable = join(nodeDirectory, target.platform === "win32" ? "node.exe" : "bin/node");
  const npmEntrypoint = join(
    nodeDirectory,
    target.platform === "win32"
      ? "node_modules/npm/bin/npm-cli.js"
      : "lib/node_modules/npm/bin/npm-cli.js",
  );
  await run(
    nodeExecutable,
    [npmEntrypoint, "ci", "--omit=dev", "--no-audit", "--no-fund", `--registry=${npmRegistry}`],
    appDirectory,
  );
  await pruneIncompatibleLinuxNativeArtifacts(appDirectory, target);

  const dshEntrypoint = join(appDirectory, "node_modules/@deepseek-ai/dsh/lib/bin.js");
  const pnpmEntrypoint = join(appDirectory, "node_modules/pnpm/bin/pnpm.cjs");
  const executableDirectory = join(appDirectory, "bin");
  await mkdir(executableDirectory, { recursive: true });
  let pnpmLauncher: string;
  if (target.platform === "win32") {
    pnpmLauncher = join(executableDirectory, "pnpm.cmd");
    await writeFile(
      pnpmLauncher,
      '@echo off\r\n"%~dp0\\..\\..\\node\\node.exe" "%~dp0\\..\\node_modules\\pnpm\\bin\\pnpm.cjs" %*\r\n',
    );
  } else {
    pnpmLauncher = join(executableDirectory, "pnpm");
    await writeFile(
      pnpmLauncher,
      '#!/bin/sh\nSCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)\nexec "$SCRIPT_DIR/../../node/bin/node" "$SCRIPT_DIR/../node_modules/pnpm/bin/pnpm.cjs" "$@"\n',
    );
    await chmod(pnpmLauncher, 0o755);
  }
  const actualNodeVersion = (await capture(nodeExecutable, ["--version"])).replace(/^v/u, "");
  const actualDshVersion = await capture(nodeExecutable, [dshEntrypoint, "--version"]);
  const actualPnpmVersion = await capture(
    nodeExecutable,
    [pnpmEntrypoint, "--version"],
    appDirectory,
  );
  if (
    actualNodeVersion !== versions.node ||
    actualDshVersion !== versions.dsh ||
    actualPnpmVersion !== versions.pnpm
  ) {
    throw new Error(
      `Runtime version mismatch: node=${actualNodeVersion}, dsh=${actualDshVersion}, pnpm=${actualPnpmVersion}`,
    );
  }

  if (target.platform !== "win32") await chmod(nodeExecutable, 0o755);
  const files: RuntimeFile[] = await Promise.all(
    [nodeExecutable, dshEntrypoint, pnpmEntrypoint, pnpmLauncher].map(async (path) => ({
      path: portablePath(assembled, path),
      sha256: await sha256(path),
    })),
  );
  const runtimeId = `dsh-${versions.dsh}-node-${versions.node}-${target.triple}`;
  const notices = await collectPackageNotices(join(appDirectory, "node_modules"));
  const noticeLines = [
    "# Bundled runtime third-party packages",
    "",
    `Node.js ${versions.node} is distributed under its upstream license in \`payload/node/LICENSE\`.`,
    "",
    "| Package | Version | Declared license |",
    "| --- | --- | --- |",
    ...notices.map(({ name, version, license }) => `| ${name} | ${version} | ${license} |`),
    "",
  ];
  await writeFile(join(assembled, "THIRD_PARTY_NOTICES.md"), noticeLines.join("\n"));

  const uncompressedArchivePath = join(assembled, "payload.tar");
  const archivePath = join(assembled, "payload.tar.gz");
  await run("tar", ["-cf", uncompressedArchivePath, "-C", assembled, "payload"]);
  await pipeline(
    createReadStream(uncompressedArchivePath),
    createGzip({ level: zlibConstants.Z_BEST_COMPRESSION }),
    createWriteStream(archivePath),
  );
  await rm(uncompressedArchivePath, { force: true });
  const manifest = {
    schemaVersion: 1,
    runtimeId,
    target: target.triple,
    nodeVersion: versions.node,
    dshVersion: versions.dsh,
    pnpmVersion: versions.pnpm,
    definitionSha256: definition,
    nodeExecutable: portablePath(assembled, nodeExecutable),
    dshEntrypoint: portablePath(assembled, dshEntrypoint),
    pnpmEntrypoint: portablePath(assembled, pnpmEntrypoint),
    archive: {
      path: portablePath(assembled, archivePath),
      sha256: await sha256(archivePath),
    },
    files,
  };
  await writeFile(join(assembled, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
  await rm(payload, { recursive: true, force: true });

  await mkdir(dirname(output), { recursive: true });
  await rm(output, { recursive: true, force: true });
  await cp(assembled, output, { recursive: true, verbatimSymlinks: true });
  await rm(temporary, { recursive: true, force: true });
  console.log(`Prepared ${runtimeId} at ${output}`);
}

await main();
