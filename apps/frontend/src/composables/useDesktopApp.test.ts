import { describe, expect, it } from "vitest";

import type { WindowSnapshot } from "@/types/desktop";

import { resolveFrameUrl } from "./useDesktopApp";

const window: WindowSnapshot = {
  label: "main",
  url: "http://127.0.0.1:3080",
  status: "running",
};

describe("resolveFrameUrl", () => {
  it("waits until the DSH service is running", () => {
    expect(resolveFrameUrl(window, "starting")).toBe("about:blank");
    expect(resolveFrameUrl(window, "unreachable")).toBe("about:blank");
    expect(resolveFrameUrl(window, "failed")).toBe("about:blank");
  });

  it("loads the window URL after the DSH service is running", () => {
    expect(resolveFrameUrl(window, "running")).toBe(window.url);
  });
});
