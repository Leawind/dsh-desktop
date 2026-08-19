import assert from "node:assert/strict";
import { describe, it } from "node:test";

import {
  addChangelogVersion,
  addCompatibilityVersion,
  updateCargoPackageVersion,
} from "./set-release-version.js";

describe("setting the release version", () => {
  it("updates only the Cargo package version", () => {
    const manifest = [
      "[package]",
      'name = "dsh-desktop"',
      'version = "0.1.0"',
      "",
      "[dependencies]",
      'example = "0.1.0"',
      "",
    ].join("\n");

    assert.equal(
      updateCargoPackageVersion(manifest, "0.2.0"),
      manifest.replace('version = "0.1.0"', 'version = "0.2.0"'),
    );
  });

  it("adds a new changelog section before existing releases", () => {
    const changelog = "# 更新日志\n\n## [0.1.0]\n\n- Initial release\n";
    const updated = addChangelogVersion(changelog, "0.2.0");

    assert.match(updated, /^# 更新日志\n\n## \[0\.2\.0\]/u);
    assert.ok(updated.indexOf("## [0.2.0]") < updated.indexOf("## [0.1.0]"));
    assert.throws(() => addChangelogVersion(updated, "0.2.0"), /already contains/u);
  });

  it("carries the verified DSH version into the new app compatibility entry", () => {
    const manifest = JSON.stringify(
      {
        schemaVersion: 2,
        apps: {
          "0.1.0": "0.1.0-rc.7",
        },
      },
      null,
      2,
    );

    assert.deepEqual(JSON.parse(addCompatibilityVersion(manifest, "0.1.0", "0.2.0")), {
      schemaVersion: 2,
      apps: {
        "0.1.0": "0.1.0-rc.7",
        "0.2.0": "0.1.0-rc.7",
      },
    });
    assert.throws(
      () => addCompatibilityVersion(manifest, "0.1.1", "0.2.0"),
      /no verified DSH version/u,
    );
  });
});
