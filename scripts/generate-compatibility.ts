import { createPrivateKey, sign } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");

export function signManifest(manifest: Buffer, signingKey: string): string {
  return sign(null, manifest, createPrivateKey(signingKey)).toString("base64");
}

async function main(): Promise<void> {
  const [outputDirectory, ...extraArguments] = process.argv
    .slice(2)
    .filter((argument) => argument !== "--");
  if (!outputDirectory || extraArguments.length > 0) {
    throw new Error("Usage: generate-compatibility.ts <output-directory>");
  }
  const signingKey = process.env.COMPATIBILITY_SIGNING_KEY;
  if (!signingKey) {
    throw new Error("COMPATIBILITY_SIGNING_KEY must contain an Ed25519 private key in PEM format");
  }

  const manifest = await readFile(join(repositoryRoot, "runtime", "compatibility.json"));
  JSON.parse(manifest.toString("utf8"));
  const output = resolve(outputDirectory);
  await mkdir(output, { recursive: true });
  await Promise.all([
    writeFile(join(output, "compatibility.json"), manifest),
    writeFile(join(output, "compatibility.json.sig"), `${signManifest(manifest, signingKey)}\n`),
  ]);
}

const invokedPath = process.argv[1] ? resolve(process.argv[1]) : undefined;
if (invokedPath === fileURLToPath(import.meta.url)) await main();
