import { spawn } from "node:child_process";
import { chmod, copyFile, mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import crossSpawn from "cross-spawn";

type DistributionVariant = "bundled" | "slim";
type DistributionPlatform = "windows" | "macos" | "linux";

interface CargoMetadata {
  packages: Array<{
    name: string;
    version: string;
  }>;
  target_directory: string;
}

interface DistributionAsset {
  architecture: string;
  platform: DistributionPlatform;
  variant: DistributionVariant;
  version: string;
}

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const executableName = process.platform === "win32" ? "dsh-desktop.exe" : "dsh-desktop";
const packageInstallers = process.env.DSH_DESKTOP_PACKAGE_INSTALLER === "1";
export const installationMarkerFilename = "dsh-desktop.installed";
const installationMarker = join(root, "installers", installationMarkerFilename);

function run(command: string, args: readonly string[], env = process.env): Promise<void> {
  return new Promise((resolvePromise, reject) => {
    const child = crossSpawn(command, args, {
      cwd: root,
      env,
      stdio: "inherit",
    });
    child.once("error", reject);
    child.once("exit", (code) =>
      code === 0
        ? resolvePromise()
        : reject(new Error(`${command} exited with code ${String(code)}`)),
    );
  });
}

function capture(command: string, args: readonly string[], env = process.env): Promise<string> {
  return new Promise((resolvePromise, reject) => {
    const child = spawn(command, args, { cwd: root, env, stdio: ["ignore", "pipe", "inherit"] });
    let output = "";
    child.stdout.setEncoding("utf8");
    child.stdout.on("data", (chunk: string) => (output += chunk));
    child.once("error", reject);
    child.once("exit", (code) =>
      code === 0
        ? resolvePromise(output)
        : reject(new Error(`${command} exited with code ${String(code)}`)),
    );
  });
}

function platform(): DistributionPlatform {
  if (process.platform === "win32") return "windows";
  if (process.platform === "darwin") return "macos";
  return "linux";
}

function architecture(): string {
  if (process.arch === "x64") return "x86_64";
  if (process.arch === "arm64") return "aarch64";
  throw new Error(`Unsupported architecture: ${process.arch}`);
}

export function portableExecutableFilename(asset: DistributionAsset): string {
  return `dsh-desktop-${asset.version}-${asset.variant}-${asset.platform}-${asset.architecture}-portable${
    asset.platform === "windows" ? ".exe" : ""
  }`;
}

export function installerFilename(asset: DistributionAsset): string {
  const extension =
    asset.platform === "windows" ? "msi" : asset.platform === "macos" ? "dmg" : "deb";
  return `dsh-desktop-${asset.version}-${asset.variant}-${asset.platform}-${asset.architecture}.${extension}`;
}

export function validateMsiVersion(version: string): void {
  const match = /^(\d+)\.(\d+)\.(\d+)(?:-(\d+))?$/.exec(version);
  if (!match) {
    throw new Error(
      `Windows MSI requires a release version in major.minor.patch or major.minor.patch-number form, got ${version}.`,
    );
  }
  const preRelease = match[4];
  if (preRelease !== undefined && Number(preRelease) > 65_535) {
    throw new Error(`Windows MSI pre-release number must not exceed 65535, got ${preRelease}.`);
  }
}

export function msiBuildArguments(
  asset: DistributionAsset,
  executable: string,
  artifacts: string,
): string[] {
  validateMsiVersion(asset.version);
  return [
    "build",
    "installers/windows.wxs",
    "-d",
    `Version=${asset.version}`,
    "-d",
    `Source=${executable}`,
    "-d",
    `MarkerSource=${installationMarker}`,
    "-o",
    join(artifacts, installerFilename(asset)),
  ];
}

async function packageDeb(
  asset: DistributionAsset,
  executable: string,
  artifacts: string,
): Promise<void> {
  const staging = await mkdtemp(join(tmpdir(), "dsh-desktop-deb-"));
  const packageName = `dsh-desktop-${asset.variant}`;
  try {
    await chmod(staging, 0o755);
    await createDebDirectory(staging, "DEBIAN");
    await writeFile(
      join(staging, "DEBIAN", "control"),
      [
        `Package: ${packageName}`,
        `Version: ${asset.version}`,
        `Architecture: ${asset.architecture === "x86_64" ? "amd64" : "arm64"}`,
        "Maintainer: Leawind",
        `Conflicts: dsh-desktop-${asset.variant === "bundled" ? "slim" : "bundled"}`,
        `Replaces: dsh-desktop-${asset.variant === "bundled" ? "slim" : "bundled"}`,
        "Description: Desktop host for DeepSeek Harness",
        " DSH Desktop opens DeepSeek Harness in a native desktop window.",
        "",
      ].join("\n"),
    );
    const binary = join(await createDebDirectory(staging, "usr", "bin"), "dsh-desktop");
    await copyFile(executable, binary);
    await chmod(binary, 0o755);
    const marker = join(dirname(binary), installationMarkerFilename);
    await copyFile(installationMarker, marker);
    await chmod(marker, 0o644);

    const icon = join(
      await createDebDirectory(staging, "usr", "share", "icons", "hicolor", "512x512", "apps"),
      "dsh-desktop.png",
    );
    await copyFile(join(root, "apps", "desktop", "icons", "icon.png"), icon);
    await chmod(icon, 0o644);

    const desktopEntry = join(
      await createDebDirectory(staging, "usr", "share", "applications"),
      "io.github.leawind.dsh-desktop.desktop",
    );
    await writeFile(
      desktopEntry,
      [
        "[Desktop Entry]",
        "Type=Application",
        "Name=DSH Desktop",
        "Comment=Desktop host for DeepSeek Harness",
        "Exec=dsh-desktop",
        "Icon=dsh-desktop",
        "Terminal=false",
        "Categories=Development;Utility;",
        "",
      ].join("\n"),
    );
    await chmod(desktopEntry, 0o644);

    await run("dpkg-deb", [
      "--build",
      "--root-owner-group",
      staging,
      join(artifacts, installerFilename(asset)),
    ]);
  } finally {
    await rm(staging, { force: true, recursive: true });
  }
}

async function createDebDirectory(
  staging: string,
  ...segments: readonly string[]
): Promise<string> {
  const directories = segments.reduce<string[]>((all, segment) => {
    all.push(join(all.at(-1) ?? staging, segment));
    return all;
  }, []);
  await Promise.all(
    directories.map(async (directory) => {
      await mkdir(directory, { recursive: true });
      await chmod(directory, 0o755);
    }),
  );
  return directories.at(-1) ?? staging;
}

async function packageDmg(
  asset: DistributionAsset,
  executable: string,
  artifacts: string,
): Promise<void> {
  const staging = await mkdtemp(join(tmpdir(), "dsh-desktop-dmg-"));
  try {
    const app = join(staging, "DSH Desktop.app");
    const contents = join(app, "Contents");
    const macOs = join(contents, "MacOS");
    const resources = join(contents, "Resources");
    await mkdir(macOs, { recursive: true });
    await mkdir(resources, { recursive: true });
    const bundledExecutable = join(macOs, "dsh-desktop");
    await copyFile(executable, bundledExecutable);
    await chmod(bundledExecutable, 0o755);
    const marker = join(macOs, installationMarkerFilename);
    await copyFile(installationMarker, marker);
    await chmod(marker, 0o644);
    await createMacIcon(resources);
    await writeFile(
      join(contents, "Info.plist"),
      [
        '<?xml version="1.0" encoding="UTF-8"?>',
        '<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">',
        '<plist version="1.0">',
        "<dict>",
        "  <key>CFBundleDisplayName</key>",
        "  <string>DSH Desktop</string>",
        "  <key>CFBundleExecutable</key>",
        "  <string>dsh-desktop</string>",
        "  <key>CFBundleIconFile</key>",
        "  <string>icon</string>",
        "  <key>CFBundleIdentifier</key>",
        "  <string>io.github.leawind.dsh-desktop</string>",
        "  <key>CFBundleName</key>",
        "  <string>DSH Desktop</string>",
        "  <key>CFBundlePackageType</key>",
        "  <string>APPL</string>",
        "  <key>CFBundleShortVersionString</key>",
        `  <string>${asset.version}</string>`,
        "  <key>CFBundleVersion</key>",
        `  <string>${asset.version}</string>`,
        "  <key>LSMinimumSystemVersion</key>",
        "  <string>11.0</string>",
        "</dict>",
        "</plist>",
        "",
      ].join("\n"),
    );
    const signingIdentity = process.env.APPLE_SIGNING_IDENTITY;
    if (signingIdentity) {
      await run("codesign", ["--force", "--deep", "--sign", signingIdentity, app]);
    }
    await run("hdiutil", [
      "create",
      "-volname",
      "DSH Desktop",
      "-srcfolder",
      app,
      "-ov",
      "-format",
      "UDZO",
      join(artifacts, installerFilename(asset)),
    ]);
  } finally {
    await rm(staging, { force: true, recursive: true });
  }
}

async function createMacIcon(resources: string): Promise<void> {
  const iconset = join(resources, "icon.iconset");
  const source = join(root, "apps", "desktop", "icons", "icon.png");
  const sizes = [
    [16, "icon_16x16.png"],
    [32, "icon_16x16@2x.png"],
    [32, "icon_32x32.png"],
    [64, "icon_32x32@2x.png"],
    [128, "icon_128x128.png"],
    [256, "icon_128x128@2x.png"],
    [256, "icon_256x256.png"],
    [512, "icon_256x256@2x.png"],
    [512, "icon_512x512.png"],
    [1024, "icon_512x512@2x.png"],
  ] as const;
  await mkdir(iconset, { recursive: true });
  await Promise.all(
    sizes.map(([size, name]) =>
      run("sips", ["-z", String(size), String(size), source, "--out", join(iconset, name)]),
    ),
  );
  await run("iconutil", ["-c", "icns", iconset, "-o", join(resources, "icon.icns")]);
}

async function packageMsi(
  asset: DistributionAsset,
  executable: string,
  artifacts: string,
): Promise<void> {
  await run("wix", msiBuildArguments(asset, executable, artifacts));
}

async function packageInstaller(
  asset: DistributionAsset,
  executable: string,
  artifacts: string,
): Promise<void> {
  switch (asset.platform) {
    case "linux":
      await packageDeb(asset, executable, artifacts);
      return;
    case "macos":
      await packageDmg(asset, executable, artifacts);
      return;
    case "windows":
      await packageMsi(asset, executable, artifacts);
      return;
  }
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
    DSH_DESKTOP_VARIANT: variant,
  };
  await run("cargo", ["build", "--release", "--manifest-path", "apps/desktop/Cargo.toml"], env);
  const metadata = JSON.parse(
    await capture(
      "cargo",
      [
        "metadata",
        "--manifest-path",
        "apps/desktop/Cargo.toml",
        "--format-version",
        "1",
        "--no-deps",
      ],
      env,
    ),
  ) as CargoMetadata;
  const desktopPackage = metadata.packages.find(({ name }) => name === "dsh-desktop");
  if (!desktopPackage) throw new Error("Cargo metadata does not contain the dsh-desktop package.");

  const executable = join(metadata.target_directory, "release", executableName);
  const artifacts = join(metadata.target_directory, "release", "artifacts");
  const asset: DistributionAsset = {
    architecture: architecture(),
    platform: platform(),
    variant,
    version: desktopPackage.version,
  };
  await mkdir(artifacts, { recursive: true });
  const portableExecutable = join(artifacts, portableExecutableFilename(asset));
  await rm(portableExecutable, { force: true });
  await copyFile(executable, portableExecutable);
  if (packageInstallers) {
    const installer = join(artifacts, installerFilename(asset));
    await rm(installer, { force: true });
    await packageInstaller(asset, executable, artifacts);
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await main();
}
