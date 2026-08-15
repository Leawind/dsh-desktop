import { mkdir, readdir, rename, rm } from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawn } from "node:child_process";

type DistributionVariant = "bundled" | "slim";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const variant = process.argv[2] as DistributionVariant | undefined;

if (variant !== "bundled" && variant !== "slim") {
  throw new Error("Usage: label-bundles.ts <bundled|slim>");
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

function variantName(name: string): string | undefined {
  if (name.includes("-bundled.") || name.includes("-slim.")) return undefined;
  if (name.endsWith(".sig")) {
    const unsigned = variantName(name.slice(0, -4));
    return unsigned ? `${unsigned}.sig` : undefined;
  }
  const extensions = [
    ".AppImage.tar.gz",
    ".app.tar.gz",
    ".AppImage",
    ".deb",
    ".rpm",
    ".msi",
    ".exe",
    ".dmg",
  ];
  const extension = extensions.find((candidate) => name.endsWith(candidate));
  if (!extension) return undefined;
  return `${name.slice(0, -extension.length)}-${variant}${extension}`;
}

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
const installersDirectory = join(bundleDirectory, "installers");
const formatDirectories = (await readdir(bundleDirectory, { withFileTypes: true })).filter(
  (entry) => entry.isDirectory() && entry.name !== "installers",
);
const entries = (
  await Promise.all(
    formatDirectories.map(async (formatDirectory) => {
      const directory = join(bundleDirectory, formatDirectory.name);
      return (await readdir(directory, { withFileTypes: true })).map((entry) => ({
        directory,
        entry,
      }));
    }),
  )
).flat();
const renames = entries.flatMap(({ directory, entry }) => {
  if (!entry.isFile()) return [];
  const labeledName = variantName(entry.name);
  if (!labeledName) return [];
  return [
    {
      source: join(directory, entry.name),
      destination: join(installersDirectory, labeledName),
    },
  ];
});
if (renames.length === 0)
  throw new Error(`No unlabeled installer bundles found in ${bundleDirectory}`);
await mkdir(installersDirectory, { recursive: true });
await Promise.all(
  renames.map(async ({ source, destination }) => {
    await rm(destination, { force: true });
    await rename(source, destination);
  }),
);
for (const { source, destination } of renames) {
  console.log(`${basename(source)} -> ${basename(destination)}`);
}
