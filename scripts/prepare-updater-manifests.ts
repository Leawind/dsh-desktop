import { readFile, readdir, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export type DistributionVariant = "bundled" | "slim";
export type UpdateTarget = "linux-x86_64" | "windows-x86_64" | "darwin-x86_64";

interface UpdatePlatform {
  signature: string;
  url: string;
}

export interface UpdateManifest {
  version: string;
  notes: string;
  platforms: Partial<Record<UpdateTarget, UpdatePlatform>>;
}

const variants: readonly DistributionVariant[] = ["bundled", "slim"];

function option(name: string): string | undefined {
  const index = process.argv.indexOf(`--${name}`);
  return index < 0 ? undefined : process.argv[index + 1];
}

function requireOption(name: string): string {
  const value = option(name);
  if (!value) throw new Error(`Missing required --${name} option`);
  return value;
}

function versionFromTag(tag: string): string {
  const match = /^v(\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)$/u.exec(tag);
  if (!match?.[1]) throw new Error(`Release tag must use the form v<version>, received: ${tag}`);
  return match[1];
}

function updateAssetName(
  version: string,
  variant: DistributionVariant,
  target: UpdateTarget,
): string {
  const prefix = `dsh-desktop-${version}-${variant}-`;
  switch (target) {
    case "linux-x86_64":
      return `${prefix}linux-x86_64.AppImage`;
    case "windows-x86_64":
      return `${prefix}windows-x86_64-setup.exe`;
    case "darwin-x86_64":
      return `${prefix}macos-x86_64.app.tar.gz`;
  }
}

async function signatureFor(directory: string, assetName: string): Promise<string> {
  const signaturePath = join(directory, `${assetName}.sig`);
  const signature = await readFile(signaturePath, "utf8").catch(() => {
    throw new Error(`Updater signature is missing for ${assetName}`);
  });
  const value = signature.trim();
  if (!value) throw new Error(`Updater signature is empty for ${assetName}`);
  return value;
}

export async function createUpdaterManifest(
  version: string,
  variant: DistributionVariant,
  notes: string,
  assetsDirectory: string,
  repository: string,
): Promise<UpdateManifest> {
  const names = new Set(await readdir(assetsDirectory));
  const entries = await Promise.all(
    (["linux-x86_64", "windows-x86_64", "darwin-x86_64"] as const).map(async (target) => {
      const assetName = updateAssetName(version, variant, target);
      if (!names.has(assetName)) return undefined;
      return [
        target,
        {
          signature: await signatureFor(assetsDirectory, assetName),
          url: `https://github.com/${repository}/releases/download/v${version}/${encodeURIComponent(assetName)}`,
        },
      ] as const;
    }),
  );
  const platforms = Object.fromEntries(entries.filter((entry) => entry !== undefined));
  if (Object.keys(platforms).length === 0) {
    throw new Error(`No updater artifacts found for ${variant}`);
  }
  return { version, notes, platforms };
}

async function main(): Promise<void> {
  const tag = requireOption("tag");
  const version = versionFromTag(tag);
  const notes = await readFile(resolve(requireOption("notes")), "utf8");
  const assetsDirectory = resolve(requireOption("assets"));
  const outputDirectory = resolve(requireOption("output"));
  const repository = option("repository") ?? "Leawind/dsh-desktop";

  const manifests = await Promise.all(
    variants.map(
      async (variant) =>
        [
          variant,
          await createUpdaterManifest(version, variant, notes, assetsDirectory, repository),
        ] as const,
    ),
  );
  await Promise.all(
    manifests.map(([variant, manifest]) =>
      writeFile(
        join(outputDirectory, `latest-${variant}.json`),
        `${JSON.stringify(manifest, null, 2)}\n`,
      ),
    ),
  );
}

const invokedPath = process.argv[1] ? resolve(process.argv[1]) : undefined;
if (invokedPath === fileURLToPath(import.meta.url)) await main();
