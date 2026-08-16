import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { addChangelogVersion, updateCargoPackageVersion } from "./set-release-version.js";

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
});
