import { afterEach, describe, expect, it, vi } from "vitest";

import { desktopBridge } from "./desktop";

afterEach(() => vi.unstubAllGlobals());

describe("startWindowHeartbeat", () => {
  it("sends a lightweight heartbeat every 250 milliseconds and stops cleanly", () => {
    let callback: (() => void) | undefined;
    const clearInterval = vi.fn();
    vi.stubGlobal("window", {
      location: { search: "?token=token&window=main", pathname: "/" },
      setInterval: vi.fn((next: () => void, delay: number) => {
        callback = next;
        expect(delay).toBe(250);
        return 1;
      }),
      clearInterval,
    });
    const fetch = vi.fn().mockResolvedValue({
      json: async () => ({ value: null }),
      ok: true,
    });
    vi.stubGlobal("fetch", fetch);

    const stop = desktopBridge.startWindowHeartbeat();
    callback?.();

    expect(fetch).toHaveBeenCalledWith("/api/command?window=main", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-DSH-Desktop-Token": "token",
      },
      body: JSON.stringify({ name: "heartbeat" }),
    });

    stop();
    expect(clearInterval).toHaveBeenCalledWith(1);
  });
});
