import type { UnlistenFn } from "@tauri-apps/api/event";
import { computed, onBeforeUnmount, readonly, ref } from "vue";

import { desktopBridge } from "@/bridge/desktop";
import { fallbackLocale, i18n, resolveInitialLocale } from "@/i18n";
import type {
  AppError,
  GlobalSettings,
  GlobalSettingsPatch,
  HostSnapshot,
  ServiceStatus,
  StartupAttemptFailure,
  WindowSnapshot,
  WindowStartupResult,
} from "@/types/desktop";

const emptyHost: HostSnapshot = { windows: [], endpoints: [] };

function setLocale(locale: ReturnType<typeof resolveInitialLocale>): void {
  i18n.global.locale.value = locale ?? fallbackLocale;
  document.documentElement.lang = i18n.global.locale.value;
}

export function resolveFrameUrl(window: WindowSnapshot | null, status: ServiceStatus): string {
  return status === "running" && window ? window.url : "about:blank";
}

export function useDesktopApp() {
  const settings = ref<GlobalSettings>({
    locale: null,
    dshSource: { type: "system" },
    windowStartupAttempts: [
      { type: "known-services" },
      { type: "connect-fixed", host: "127.0.0.1", port: 3080 },
      { type: "start-fixed", host: "127.0.0.1", port: 3080 },
      { type: "start-range", host: "127.0.0.1", startPort: 3081, endPort: 3090 },
    ],
  });
  const currentWindow = ref<WindowSnapshot | null>(null);
  const host = ref<HostSnapshot>(emptyHost);
  const startupStatus = ref<ServiceStatus>("starting");
  const error = ref<AppError | null>(null);
  const startupFailures = ref<StartupAttemptFailure[]>([]);
  const settingsOpen = ref(false);
  const frameRevision = ref(0);
  const unlisteners: UnlistenFn[] = [];

  const frameUrl = computed(() => resolveFrameUrl(currentWindow.value, startupStatus.value));

  async function initialize(): Promise<void> {
    try {
      const payload = await desktopBridge.initializeWindow();
      settings.value = payload.settings;
      currentWindow.value = payload.window;
      host.value = payload.host;
      setLocale(resolveInitialLocale(payload.settings.locale));

      unlisteners.push(
        await desktopBridge.onGlobalSettingsChanged((value) => {
          settings.value = value;
          setLocale(resolveInitialLocale(value.locale));
        }),
        await desktopBridge.onHostSnapshotChanged((value) => {
          host.value = value;
          syncWindowFromHost();
        }),
      );

      startupStatus.value = "starting";
      applyStartupResult(await desktopBridge.startWindow());
    } catch (cause) {
      error.value = cause as AppError;
      startupStatus.value = "failed";
    }
  }

  async function setTarget(url: string): Promise<void> {
    error.value = null;
    try {
      currentWindow.value = await desktopBridge.setWindowTarget(url);
      frameRevision.value += 1;
      host.value = await desktopBridge.getHostSnapshot();
      startupStatus.value = currentWindow.value.status;
      if (currentWindow.value.status !== "running") {
        error.value = { code: "service.error.unreachable", args: { url } };
      }
    } catch (cause) {
      error.value = cause as AppError;
    }
  }

  async function retryStartup(): Promise<void> {
    error.value = null;
    startupFailures.value = [];
    startupStatus.value = "starting";
    try {
      const result = await desktopBridge.startWindow();
      applyStartupResult(result);
      if (result.connected) frameRevision.value += 1;
    } catch (cause) {
      error.value = cause as AppError;
      startupStatus.value = "failed";
    }
  }

  async function saveGlobalSettings(patch: GlobalSettingsPatch): Promise<void> {
    error.value = null;
    try {
      settings.value = await desktopBridge.updateGlobalSettings(patch);
      setLocale(resolveInitialLocale(settings.value.locale));
    } catch (cause) {
      error.value = cause as AppError;
      throw cause;
    }
  }

  function reloadFrame(): void {
    frameRevision.value += 1;
  }

  function syncWindowFromHost(): void {
    const label = currentWindow.value?.label;
    if (!label) return;
    const updated = host.value.windows.find((window) => window.label === label);
    if (!updated) return;
    const wasRunning = currentWindow.value?.status === "running";
    currentWindow.value = updated;
    if (startupStatus.value === "starting") return;
    startupStatus.value = updated.status;
    if (wasRunning && updated.status !== "running") {
      error.value = {
        code: "service.error.connectionLost",
        args: { url: updated.url },
      };
    } else if (
      updated.status === "running" &&
      error.value?.code === "service.error.connectionLost"
    ) {
      error.value = null;
    }
  }

  function applyStartupResult(result: WindowStartupResult): void {
    host.value = result.host;
    currentWindow.value = result.window;
    startupFailures.value = result.failures;
    startupStatus.value = result.connected ? "running" : "failed";
    error.value = result.connected ? null : { code: "service.error.allAttemptsFailed" };
  }

  onBeforeUnmount(() => {
    for (const unlisten of unlisteners) unlisten();
  });

  return {
    settings: readonly(settings),
    currentWindow: readonly(currentWindow),
    host: readonly(host),
    startupStatus: readonly(startupStatus),
    error: readonly(error),
    startupFailures: readonly(startupFailures),
    settingsOpen,
    frameRevision: readonly(frameRevision),
    frameUrl,
    initialize,
    setTarget,
    retryStartup,
    saveGlobalSettings,
    reloadFrame,
  };
}
