import assert from "node:assert/strict";
import test from "node:test";
import { resolve } from "node:path";
import {
  installationMarkerFilename,
  installerFilename,
  msiBuildArguments,
  portableExecutableFilename,
  validateMsiVersion,
} from "./build.js";

const bundledWindows = {
  architecture: "x86_64",
  platform: "windows" as const,
  variant: "bundled" as const,
  version: "0.2.2",
};

test("distribution asset names include the version, variant, platform, and architecture", () => {
  assert.equal(
    portableExecutableFilename(bundledWindows),
    "dsh-desktop-0.2.2-bundled-windows-x86_64-portable.exe",
  );
  assert.equal(installerFilename(bundledWindows), "dsh-desktop-0.2.2-bundled-windows-x86_64.msi");
  assert.equal(
    installerFilename({ ...bundledWindows, platform: "macos" }),
    "dsh-desktop-0.2.2-bundled-macos-x86_64.dmg",
  );
  assert.equal(
    installerFilename({ ...bundledWindows, platform: "linux" }),
    "dsh-desktop-0.2.2-bundled-linux-x86_64.deb",
  );
});

test("Windows MSI version accepts only a numeric pre-release suffix", () => {
  assert.doesNotThrow(() => validateMsiVersion("0.2.2"));
  assert.doesNotThrow(() => validateMsiVersion("0.2.2-2"));
  assert.throws(() => validateMsiVersion("0.2.2-rc.2"));
  assert.throws(() => validateMsiVersion("0.2.2-65536"));
});

test("Windows MSI passes each WiX preprocessor variable as an option value", () => {
  assert.deepEqual(msiBuildArguments(bundledWindows, "C:/build/dsh-desktop.exe", "C:/artifacts"), [
    "build",
    "installers/windows.wxs",
    "-d",
    "Version=0.2.2",
    "-d",
    "Source=C:/build/dsh-desktop.exe",
    "-d",
    `MarkerSource=${resolve("installers", installationMarkerFilename)}`,
    "-o",
    "C:/artifacts/dsh-desktop-0.2.2-bundled-windows-x86_64.msi",
  ]);
});
