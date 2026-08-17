import assert from "node:assert/strict";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, it } from "node:test";

import { createUpdaterManifest } from "./prepare-updater-manifests.js";

describe("updater manifest preparation", () => {
  it("keeps bundled and slim update channels separate", async () => {
    const assets = await mkdtemp(join(tmpdir(), "dsh-desktop-updater-"));
    const bundledAsset = "dsh-desktop-0.2.0-bundled-linux-x86_64.AppImage";
    const slimAsset = "dsh-desktop-0.2.0-slim-windows-x86_64-setup.exe";
    await Promise.all([
      writeFile(join(assets, bundledAsset), "bundled"),
      writeFile(join(assets, `${bundledAsset}.sig`), "bundled-signature\n"),
      writeFile(join(assets, slimAsset), "slim"),
      writeFile(join(assets, `${slimAsset}.sig`), "slim-signature\n"),
    ]);

    const bundled = await createUpdaterManifest(
      "0.2.0",
      "bundled",
      "Bundled update",
      assets,
      "Leawind/dsh-desktop",
    );
    const slim = await createUpdaterManifest(
      "0.2.0",
      "slim",
      "Slim update",
      assets,
      "Leawind/dsh-desktop",
    );

    assert.deepEqual(bundled.platforms, {
      "linux-x86_64": {
        signature: "bundled-signature",
        url: `https://github.com/Leawind/dsh-desktop/releases/download/v0.2.0/${bundledAsset}`,
      },
    });
    assert.deepEqual(slim.platforms, {
      "windows-x86_64": {
        signature: "slim-signature",
        url: `https://github.com/Leawind/dsh-desktop/releases/download/v0.2.0/${slimAsset}`,
      },
    });
  });

  it("requires a signature for every published updater artifact", async () => {
    const assets = await mkdtemp(join(tmpdir(), "dsh-desktop-updater-"));
    await writeFile(join(assets, "dsh-desktop-0.2.0-bundled-linux-x86_64.AppImage"), "bundled");

    await assert.rejects(
      createUpdaterManifest("0.2.0", "bundled", "Bundled update", assets, "Leawind/dsh-desktop"),
      /signature/i,
    );
  });
});
