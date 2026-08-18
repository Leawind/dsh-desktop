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

describe("startWindowControl", () => {
  it("closes the page when the Host sends a close event", () => {
    const listeners = new Map<string, (event: { data: string }) => void>();
    const closeWindow = vi.fn();
    vi.stubGlobal(
      "WebSocket",
      vi.fn(function (url: string) {
        expect(url).toBe("ws://127.0.0.1:1420/api/window-control?window=main&token=token");
        return {
          addEventListener: vi.fn((name, callback) => listeners.set(name, callback)),
          close: vi.fn(),
        };
      }),
    );
    vi.stubGlobal("window", {
      close: closeWindow,
      location: {
        host: "127.0.0.1:1420",
        pathname: "/",
        protocol: "http:",
        search: "?token=token&window=main",
      },
      clearTimeout: vi.fn(),
      setTimeout: vi.fn(),
    });

    desktopBridge.startWindowControl();
    listeners.get("message")?.({ data: '{"type":"close"}' });

    expect(closeWindow).toHaveBeenCalledOnce();
  });

  it("reconnects after an unexpected control-channel close", () => {
    let closeHandler: (() => void) | undefined;
    const listeners = new Map<string, () => void>();
    const setTimeout = vi.fn((callback: () => void, delay: number) => {
      closeHandler = callback;
      expect(delay).toBe(250);
      return 1;
    });
    vi.stubGlobal(
      "WebSocket",
      vi.fn(function () {
        return {
          addEventListener: vi.fn((name, callback) => listeners.set(name, callback)),
          close: vi.fn(),
        };
      }),
    );
    vi.stubGlobal("window", {
      location: { host: "127.0.0.1:1420", pathname: "/", protocol: "http:", search: "" },
      clearTimeout: vi.fn(),
      setTimeout,
    });

    desktopBridge.startWindowControl();
    listeners.get("close")?.();
    closeHandler?.();

    expect(WebSocket).toHaveBeenCalledTimes(2);
  });
});
