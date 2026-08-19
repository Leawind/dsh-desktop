import { spawn } from "node:child_process";
import { readFile, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { versionFromTag } from "./prepare-release.js";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const changelogPlaceholder = "<!-- 发布前在此填写更新日志；仅保留本注释将无法通过发布检查。 -->";

export function updateCargoPackageVersion(manifest: string, version: string): string {
  const packageStart = manifest.indexOf("[package]");
  const packageEnd = manifest.indexOf("\n[", packageStart + "[package]".length);
  if (packageStart < 0) throw new Error("Cargo.toml has no [package] section");

  const end = packageEnd < 0 ? manifest.length : packageEnd;
  const packageSection = manifest.slice(packageStart, end);
  const versionPattern = /^version[ \t]*=[ \t]*"[^"]+"[ \t]*$/gmu;
  const matches = [...packageSection.matchAll(versionPattern)];
  if (matches.length !== 1) {
    throw new Error("Cargo.toml [package] section must contain exactly one version");
  }

  const updatedSection = packageSection.replace(versionPattern, `version = "${version}"`);
  return `${manifest.slice(0, packageStart)}${updatedSection}${manifest.slice(end)}`;
}

function cargoPackageVersion(manifest: string): string {
  const packageStart = manifest.indexOf("[package]");
  const packageEnd = manifest.indexOf("\n[", packageStart + "[package]".length);
  if (packageStart < 0) throw new Error("Cargo.toml has no [package] section");

  const packageSection = manifest.slice(
    packageStart,
    packageEnd < 0 ? manifest.length : packageEnd,
  );
  const matches = [...packageSection.matchAll(/^version[ \t]*=[ \t]*"([^"]+)"[ \t]*$/gmu)];
  if (matches.length !== 1 || !matches[0]?.[1]) {
    throw new Error("Cargo.toml [package] section must contain exactly one version");
  }
  return matches[0][1];
}

export function addChangelogVersion(changelog: string, version: string): string {
  const lines = changelog.split(/\r?\n/u);
  const versionHeadings = lines
    .map((line) => /^## \[([^\]]+)\](?:\s+-\s+.+)?\s*$/u.exec(line)?.[1])
    .filter((value): value is string => value !== undefined);
  if (versionHeadings.includes(version)) {
    throw new Error(`CHANGELOG.md already contains a section for ${version}`);
  }

  const firstVersion = lines.findIndex((line) => /^## \[[^\]]+\]/u.test(line));
  const insertion = firstVersion < 0 ? lines.length : firstVersion;
  const before = lines.slice(0, insertion).join("\n").trimEnd();
  const after = lines.slice(insertion).join("\n").trimStart();
  const section = `## [${version}]\n\n${changelogPlaceholder}`;
  return `${before}\n\n${section}${after ? `\n\n${after}` : ""}\n`;
}

export function addCompatibilityVersion(
  manifest: string,
  currentAppVersion: string,
  version: string,
): string {
  const parsed: unknown = JSON.parse(manifest);
  if (
    !parsed ||
    typeof parsed !== "object" ||
    Array.isArray(parsed) ||
    (parsed as { schemaVersion?: unknown }).schemaVersion !== 2 ||
    !(parsed as { apps?: unknown }).apps ||
    typeof (parsed as { apps: unknown }).apps !== "object" ||
    Array.isArray((parsed as { apps: unknown }).apps)
  ) {
    throw new Error("runtime/compatibility.json must contain a schema version 2 apps object");
  }

  const apps = (parsed as { apps: Record<string, unknown> }).apps;
  const currentDshVersion = apps[currentAppVersion];
  if (typeof currentDshVersion !== "string" || !currentDshVersion) {
    throw new Error(
      `runtime/compatibility.json has no verified DSH version for ${currentAppVersion}`,
    );
  }
  if (Object.hasOwn(apps, version)) {
    throw new Error(`runtime/compatibility.json already contains a version for ${version}`);
  }

  apps[version] = currentDshVersion;
  return `${JSON.stringify(parsed, null, 2)}\n`;
}

async function refreshCargoLock(): Promise<void> {
  await new Promise<void>((resolvePromise, reject) => {
    const child = spawn(
      "cargo",
      [
        "metadata",
        "--manifest-path",
        "apps/desktop/Cargo.toml",
        "--format-version",
        "1",
        "--no-deps",
      ],
      { cwd: repositoryRoot, stdio: ["ignore", "ignore", "inherit"] },
    );
    child.once("error", reject);
    child.once("exit", (code) => {
      if (code === 0) resolvePromise();
      else reject(new Error(`cargo metadata exited with code ${String(code)}`));
    });
  });
}

async function main(): Promise<void> {
  const [requestedVersion, ...extraArguments] = process.argv
    .slice(2)
    .filter((argument) => argument !== "--");
  if (!requestedVersion || extraArguments.length > 0) {
    throw new Error("Usage: set-release-version.ts <version>");
  }
  const version = versionFromTag(`v${requestedVersion}`);

  const cargoPath = join(repositoryRoot, "apps", "desktop", "Cargo.toml");
  const changelogPath = join(repositoryRoot, "CHANGELOG.md");
  const compatibilityPath = join(repositoryRoot, "runtime", "compatibility.json");
  const [manifest, changelog, compatibility] = await Promise.all([
    readFile(cargoPath, "utf8"),
    readFile(changelogPath, "utf8"),
    readFile(compatibilityPath, "utf8"),
  ]);
  const currentVersion = cargoPackageVersion(manifest);
  const updatedManifest = updateCargoPackageVersion(manifest, version);
  const updatedChangelog = addChangelogVersion(changelog, version);
  const updatedCompatibility = addCompatibilityVersion(compatibility, currentVersion, version);

  await Promise.all([
    writeFile(cargoPath, updatedManifest),
    writeFile(changelogPath, updatedChangelog),
    writeFile(compatibilityPath, updatedCompatibility),
  ]);
  await refreshCargoLock();
  console.log(`Set DSH Desktop version to ${version}`);
}

const invokedPath = process.argv[1] ? resolve(process.argv[1]) : undefined;
if (invokedPath === fileURLToPath(import.meta.url)) await main();
