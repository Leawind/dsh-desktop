import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { extractReleaseNotes, versionFromTag } from "./prepare-release.js";

describe("release preparation", () => {
  it("derives the release version from a v-prefixed tag", () => {
    assert.equal(versionFromTag("v1.2.3"), "1.2.3");
    assert.equal(versionFromTag("v1.2.3-beta.1"), "1.2.3-beta.1");
    assert.throws(() => versionFromTag("1.2.3"), /v<version>/u);
  });

  it("extracts only the matching changelog section", () => {
    const changelog = [
      "# Changelog",
      "",
      "## [1.2.0] - 2026-08-16",
      "",
      "- Current release",
      "",
      "## [1.1.0]",
      "",
      "- Previous release",
      "",
    ].join("\n");

    assert.equal(extractReleaseNotes(changelog, "1.2.0"), "- Current release\n");
    assert.throws(() => extractReleaseNotes(changelog, "2.0.0"), /does not contain/u);
  });

  it("rejects a changelog section that only contains comments", () => {
    assert.throws(
      () => extractReleaseNotes("## [1.2.0]\n\n<!-- Fill this in. -->\n", "1.2.0"),
      /is empty/u,
    );
  });
});
