import { spawn } from "node:child_process";
import { mkdir, readdir, rename, rm } from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export type DistributionVariant = "bundled" | "slim";
export type ArtifactPlatform = "linux" | "windows" | "macos";
export type ArtifactArchitecture = "x86_64" | "aarch64";

interface CargoMetadata {
  target_directory: string;
  packages: Array<{
    name: string;
    version: string;
  }>;
}

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");

const artifactSuffixes = [
  ".AppImage.tar.gz",
  ".app.tar.gz",
  ".AppImage",
  ".deb",
  ".rpm",
  ".msi",
  ".exe",
  ".dmg",
] as const;

function currentPlatform(): ArtifactPlatform {
  if (process.platform === "linux") return "linux";
  if (process.platform === "win32") return "windows";
  if (process.platform === "darwin") return "macos";
  throw new Error(`Installer naming is not configured for platform ${process.platform}`);
}

function currentArchitecture(): ArtifactArchitecture {
  if (process.arch === "x64") return "x86_64";
  if (process.arch === "arm64") return "aarch64";
  throw new Error(`Installer naming is not configured for architecture ${process.arch}`);
}

export function artifactName(
  sourceName: string,
  version: string,
  variant: DistributionVariant,
  platform: ArtifactPlatform,
  architecture: ArtifactArchitecture,
): string | undefined {
  if (sourceName.endsWith(".sig")) {
    const unsigned = artifactName(
      sourceName.slice(0, -4),
      version,
      variant,
      platform,
      architecture,
    );
    return unsigned ? `${unsigned}.sig` : undefined;
  }

  const sourceSuffix = artifactSuffixes.find((candidate) => sourceName.endsWith(candidate));
  if (!sourceSuffix) return undefined;
  const outputSuffix = sourceSuffix === ".exe" ? "-setup.exe" : sourceSuffix;
  return `dsh-desktop-${version}-${variant}-${platform}-${architecture}${outputSuffix}`;
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

async function main(): Promise<void> {
  const variant = process.argv[2] as DistributionVariant | undefined;
  if (variant !== "bundled" && variant !== "slim") {
    throw new Error("Usage: label-bundles.ts <bundled|slim>");
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
  ) as CargoMetadata;
  const appPackage = metadata.packages.find((candidate) => candidate.name === "dsh-desktop");
  if (!appPackage) throw new Error("Cargo metadata does not contain the dsh-desktop package");

  const platform = currentPlatform();
  const architecture = currentArchitecture();
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
    const normalizedName = artifactName(
      entry.name,
      appPackage.version,
      variant,
      platform,
      architecture,
    );
    if (!normalizedName) return [];
    return [
      {
        source: join(directory, entry.name),
        destination: join(installersDirectory, normalizedName),
      },
    ];
  });
  if (renames.length === 0) {
    throw new Error(`No installer bundles found in ${bundleDirectory}`);
  }

  const destinations = new Set<string>();
  for (const { destination } of renames) {
    if (destinations.has(destination)) {
      throw new Error(`Multiple installer bundles map to ${basename(destination)}`);
    }
    destinations.add(destination);
  }

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
}

const invokedPath = process.argv[1] ? resolve(process.argv[1]) : undefined;
if (invokedPath === fileURLToPath(import.meta.url)) await main();
