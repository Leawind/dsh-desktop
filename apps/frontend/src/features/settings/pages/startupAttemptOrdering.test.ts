import { describe, expect, it } from "vitest";

import { moveItem } from "./startupAttemptOrdering";

describe("moveItem", () => {
  it("moves an item to an earlier position", () => {
    const items = ["known", "connect", "start"];

    expect(moveItem(items, 2, 1)).toBe(true);
    expect(items).toEqual(["known", "start", "connect"]);
  });

  it("moves an item to a later position", () => {
    const items = ["known", "connect", "start"];

    expect(moveItem(items, 0, 2)).toBe(true);
    expect(items).toEqual(["connect", "start", "known"]);
  });

  it("does not change the list for an invalid or unchanged target", () => {
    const items = ["known", "connect"];

    expect(moveItem(items, 0, 0)).toBe(false);
    expect(moveItem(items, 0, 3)).toBe(false);
    expect(items).toEqual(["known", "connect"]);
  });
});
