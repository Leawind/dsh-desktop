import { readFile, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");

export function versionFromTag(tag: string): string {
  const match = /^v(\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)$/u.exec(tag);
  if (!match?.[1]) {
    throw new Error(`Release tag must use the form v<version>, received: ${tag}`);
  }
  return match[1];
}

export function extractReleaseNotes(changelog: string, version: string): string {
  const lines = changelog.split(/\r?\n/u);
  const heading = /^## \[([^\]]+)\](?:\s+-\s+.+)?\s*$/u;
  const start = lines.findIndex((line) => heading.exec(line)?.[1] === version);
  if (start < 0) {
    throw new Error(`CHANGELOG.md does not contain a section for ${version}`);
  }

  const relativeEnd = lines.slice(start + 1).findIndex((line) => /^##\s+/u.test(line));
  const end = relativeEnd < 0 ? lines.length : start + 1 + relativeEnd;
  const notes = lines
    .slice(start + 1, end)
    .join("\n")
    .trim();
  const visibleNotes = notes.replace(/<!--[\s\S]*?-->/gu, "").trim();
  if (!visibleNotes) {
    throw new Error(`CHANGELOG.md section for ${version} is empty`);
  }
  return `${notes}\n`;
}

async function cargoVersion(path: string): Promise<string> {
  const contents = await readFile(path, "utf8");
  const packageStart = contents.indexOf("[package]");
  const packageEnd = contents.indexOf("\n[", packageStart + "[package]".length);
  const packageSection =
    packageStart < 0
      ? undefined
      : contents.slice(packageStart, packageEnd < 0 ? undefined : packageEnd);
  const version = packageSection
    ? /^version\s*=\s*"([^"]+)"\s*$/mu.exec(packageSection)?.[1]
    : null;
  if (!version) throw new Error(`${path} has no package version`);
  return version;
}

async function main(): Promise<void> {
  const [tag, output] = process.argv.slice(2).filter((argument) => argument !== "--");
  if (!tag || !output) {
    throw new Error("Usage: prepare-release.ts <tag> <release-notes-output>");
  }

  const version = versionFromTag(tag);
  const appVersion = await cargoVersion(join(repositoryRoot, "apps", "desktop", "Cargo.toml"));
  if (appVersion !== version) {
    throw new Error(`Release tag version ${version} does not match Cargo version ${appVersion}`);
  }

  const changelog = await readFile(join(repositoryRoot, "CHANGELOG.md"), "utf8");
  const notes = extractReleaseNotes(changelog, version);
  await writeFile(resolve(output), notes);
  console.log(`Prepared release notes for ${tag}`);
}

const invokedPath = process.argv[1] ? resolve(process.argv[1]) : undefined;
if (invokedPath === fileURLToPath(import.meta.url)) await main();
