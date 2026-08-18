import { describe, expect, it } from "vitest";

import type { WindowSnapshot } from "@/types/desktop";

import { resolveFrameUrl, resolveRefreshAction } from "./useDesktopApp";

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

describe("resolveRefreshAction", () => {
  it("refreshes a connected DSH page", () => {
    expect(resolveRefreshAction("running")).toBe("refresh");
  });

  it("retries startup after a connection failure", () => {
    expect(resolveRefreshAction("failed")).toBe("retry");
    expect(resolveRefreshAction("unreachable")).toBe("retry");
  });

  it("does nothing while the service state is transitional", () => {
    expect(resolveRefreshAction("starting")).toBeNull();
    expect(resolveRefreshAction("stopping")).toBeNull();
    expect(resolveRefreshAction("restarting")).toBeNull();
  });
});
