import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { artifactName } from "./label-bundles.js";

describe("installer artifact naming", () => {
  it("normalizes Linux installer names", () => {
    assert.equal(
      artifactName("DSH Desktop_0.1.0_amd64.AppImage", "0.1.0", "bundled", "linux", "x86_64"),
      "dsh-desktop-0.1.0-bundled-linux-x86_64.AppImage",
    );
    assert.equal(
      artifactName("DSH Desktop_0.1.0_amd64.deb", "0.1.0", "slim", "linux", "x86_64"),
      "dsh-desktop-0.1.0-slim-linux-x86_64.deb",
    );
    assert.equal(
      artifactName("DSH Desktop-0.1.0-1.x86_64.rpm", "0.1.0", "bundled", "linux", "x86_64"),
      "dsh-desktop-0.1.0-bundled-linux-x86_64.rpm",
    );
  });

  it("normalizes Windows and macOS installer names", () => {
    assert.equal(
      artifactName("DSH Desktop_0.1.0_x64_en-US.msi", "0.1.0", "slim", "windows", "x86_64"),
      "dsh-desktop-0.1.0-slim-windows-x86_64.msi",
    );
    assert.equal(
      artifactName("DSH Desktop_0.1.0_x64-setup.exe", "0.1.0", "bundled", "windows", "x86_64"),
      "dsh-desktop-0.1.0-bundled-windows-x86_64-setup.exe",
    );
    assert.equal(
      artifactName("DSH Desktop_0.1.0_x64.dmg", "0.1.0", "bundled", "macos", "x86_64"),
      "dsh-desktop-0.1.0-bundled-macos-x86_64.dmg",
    );
  });

  it("preserves updater suffixes and signatures", () => {
    assert.equal(
      artifactName("DSH Desktop.AppImage.tar.gz.sig", "0.2.0", "slim", "linux", "aarch64"),
      "dsh-desktop-0.2.0-slim-linux-aarch64.AppImage.tar.gz.sig",
    );
    assert.equal(artifactName("DSH Desktop.txt", "0.1.0", "slim", "linux", "x86_64"), undefined);
  });
});
