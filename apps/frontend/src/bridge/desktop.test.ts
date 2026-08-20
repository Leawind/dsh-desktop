import { afterEach, describe, expect, it, vi } from "vitest";

import type { BootstrapPayload } from "@/types/desktop";

import { desktopBridge } from "./desktop";

afterEach(() => vi.unstubAllGlobals());

const desktopSnapshot = {
  app: {
    name: "DSH Desktop",
    version: "0.2.3",
    identifier: "io.github.leawind.dsh-desktop",
  },
  settings: {
    locale: "zh-CN",
    dshSource: { type: "npx", version: "0.1.0" },
    dshHome: { type: "custom", path: "/tmp/dsh" },
    windowStartupAttempts: [
      { type: "start-range", host: "127.0.0.1", startPort: 3080, endPort: 3090 },
    ],
    managedServiceIdleTimeoutSeconds: 120,
  },
  distribution: {
    variant: "bundled",
    builtInRuntime: {
      runtimeId: "runtime-id",
      nodeVersion: "24.18.1",
      dshVersion: "0.1.0",
      pnpmVersion: "11.7.0",
      installed: true,
    },
  },
  window: {
    label: "main",
    url: "http://127.0.0.1:3080",
    status: "running",
  },
  host: {
    windows: [],
    endpoints: [
      {
        url: "http://127.0.0.1:3080",
        status: "running",
        ownership: "managed",
        connectedWindows: 1,
        pid: 42,
        runtimeVersion: "0.1.0",
        lastError: null,
        known: true,
        canStop: true,
        canRestart: true,
        logs: ["started"],
      },
    ],
  },
  systemColorScheme: "dark",
} satisfies BootstrapPayload;

describe("getDesktopSnapshot", () => {
  it("uses the snapshot command and decodes the shared wire shape", async () => {
    vi.stubGlobal("window", {
      location: { search: "?token=token&window=main", pathname: "/" },
    });
    const fetch = vi.fn().mockResolvedValue({
      json: async () => ({ value: desktopSnapshot }),
      ok: true,
    });
    vi.stubGlobal("fetch", fetch);

    await expect(desktopBridge.getDesktopSnapshot()).resolves.toEqual(desktopSnapshot);
    expect(fetch).toHaveBeenCalledWith("/api/command?window=main", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-DSH-Desktop-Token": "token",
      },
      body: JSON.stringify({ name: "get_desktop_snapshot", args: undefined }),
    });
  });
});

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

    expect(fetch).toHaveBeenCalledWith("/api/heartbeat?window=main", {
      method: "POST",
      headers: {
        "X-DSH-Desktop-Token": "token",
      },
    });

    stop();
    expect(clearInterval).toHaveBeenCalledWith(1);
  });
});

describe("onDesktopSnapshotChanged", () => {
  it("does not overlap a slow snapshot request", async () => {
    let callback: (() => void) | undefined;
    let resolveFirst: ((value: { json: () => Promise<object>; ok: boolean }) => void) | undefined;
    vi.stubGlobal("window", {
      location: { search: "?token=token&window=main", pathname: "/" },
      setInterval: vi.fn((next: () => void) => {
        callback = next;
        return 1;
      }),
      clearInterval: vi.fn(),
    });
    const response = {
      json: async () => ({ value: { host: { endpoints: [], windows: [] } } }),
      ok: true,
    };
    const fetch = vi
      .fn()
      .mockImplementationOnce(
        () =>
          new Promise<{ json: () => Promise<object>; ok: boolean }>((resolve) => {
            resolveFirst = resolve;
          }),
      )
      .mockResolvedValue(response);
    vi.stubGlobal("fetch", fetch);

    const stop = await desktopBridge.onDesktopSnapshotChanged(() => {});
    expect(fetch).toHaveBeenCalledTimes(1);

    callback?.();
    expect(fetch).toHaveBeenCalledTimes(1);

    resolveFirst?.(response);
    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();
    callback?.();
    expect(fetch).toHaveBeenCalledTimes(2);

    stop();
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
