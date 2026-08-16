import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

import type {
  AppError,
  BootstrapPayload,
  GlobalSettings,
  GlobalSettingsPatch,
  HostSnapshot,
  WindowStartupResult,
  WindowSnapshot,
} from "@/types/desktop";

function normalizeError(error: unknown): AppError {
  if (typeof error === "object" && error !== null && "code" in error) {
    const candidate = error as Partial<AppError>;
    if (typeof candidate.code === "string") {
      return candidate as AppError;
    }
  }
  return {
    code: "app.error.unknown",
    technicalDetails: error instanceof Error ? error.message : String(error),
  };
}

async function command<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(name, args);
  } catch (error) {
    throw normalizeError(error);
  }
}

export const desktopBridge = {
  initializeWindow: (): Promise<BootstrapPayload> => command("initialize_window"),

  focusWindow: (label: string): Promise<void> => command("focus_app_window", { label }),

  closeWindow: (label: string): Promise<void> => command("close_app_window", { label }),

  getHostSnapshot: (): Promise<HostSnapshot> => command("get_host_snapshot"),

  startWindow: (): Promise<WindowStartupResult> => command("start_window"),

  setWindowTarget: (url: string): Promise<WindowSnapshot> => command("set_window_target", { url }),

  stopService: (url: string): Promise<HostSnapshot> => command("stop_service", { url }),

  restartService: (url: string): Promise<HostSnapshot> => command("restart_service", { url }),

  updateGlobalSettings: (patch: GlobalSettingsPatch): Promise<GlobalSettings> =>
    command("update_global_settings", { patch }),

  onHostSnapshotChanged: (listener: (snapshot: HostSnapshot) => void): Promise<UnlistenFn> =>
    listen<HostSnapshot>("host-snapshot-changed", (event) => listener(event.payload)),

  onGlobalSettingsChanged: (listener: (settings: GlobalSettings) => void): Promise<UnlistenFn> =>
    listen<GlobalSettings>("global-settings-changed", (event) => listener(event.payload)),

  window: {
    minimize: (): Promise<void> => getCurrentWindow().minimize(),
    toggleMaximize: (): Promise<void> => getCurrentWindow().toggleMaximize(),
    close: (): Promise<void> => getCurrentWindow().close(),
    startDragging: (): Promise<void> => getCurrentWindow().startDragging(),
    setTitle: (title: string): Promise<void> =>
      isTauri() ? getCurrentWindow().setTitle(title) : Promise.resolve(),
  },
};
