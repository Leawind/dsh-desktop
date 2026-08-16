import type { UnlistenFn } from "@tauri-apps/api/event";
import { computed, onBeforeUnmount, readonly, ref } from "vue";

import { desktopBridge } from "@/bridge/desktop";
import { applyLocale, resolveInitialLocale } from "@/i18n";
import type {
  AppMetadataSnapshot,
  AppError,
  DistributionSnapshot,
  GlobalSettings,
  GlobalSettingsPatch,
  HostSnapshot,
  ServiceStatus,
  StartupAttemptFailure,
  WindowSnapshot,
  WindowStartupResult,
} from "@/types/desktop";

const emptyHost: HostSnapshot = { windows: [], endpoints: [] };
export const desktopWindowTitle = "Deepseek Harness Desktop";

export function resolveFrameUrl(window: WindowSnapshot | null, status: ServiceStatus): string {
  return status === "running" && window ? window.url : "about:blank";
}

export function resolveRefreshAction(status: ServiceStatus): "refresh" | "retry" | null {
  if (status === "running") return "refresh";
  if (status === "failed" || status === "unreachable") return "retry";
  return null;
}

export function resolveWindowTitle(window: WindowSnapshot | null, status: ServiceStatus): string {
  if (status !== "running" || !window) return desktopWindowTitle;

  try {
    const endpoint = new URL(window.url);
    const port = endpoint.port || (endpoint.protocol === "http:" ? "80" : "443");
    return `${desktopWindowTitle} - ${endpoint.hostname}:${port}`;
  } catch {
    return desktopWindowTitle;
  }
}

export function useDesktopApp() {
  const appMetadata = ref<AppMetadataSnapshot>({
    name: "DSH Desktop",
    version: "",
    identifier: "io.github.leawind.dsh-desktop",
  });
  const settings = ref<GlobalSettings>({
    locale: null,
    pageScalePercent: 100,
    dshSource: { type: "system" },
    dshHome: { type: "environment" },
    windowStartupAttempts: [
      { type: "known-services" },
      { type: "connect-fixed", host: "127.0.0.1", port: 3080 },
      { type: "start-range", host: "127.0.0.1", startPort: 3080, endPort: 3090 },
    ],
    managedServiceIdleTimeoutSeconds: 0,
  });
  const currentWindow = ref<WindowSnapshot | null>(null);
  const distribution = ref<DistributionSnapshot>({ variant: "slim", builtInRuntime: null });
  const host = ref<HostSnapshot>(emptyHost);
  const startupStatus = ref<ServiceStatus>("starting");
  const error = ref<AppError | null>(null);
  const startupFailures = ref<StartupAttemptFailure[]>([]);
  const settingsOpen = ref(false);
  const frameRevision = ref(0);
  const unlisteners: UnlistenFn[] = [];

  const frameUrl = computed(() => resolveFrameUrl(currentWindow.value, startupStatus.value));
  const refreshAction = computed(() => resolveRefreshAction(startupStatus.value));
  const windowTitle = computed(() => resolveWindowTitle(currentWindow.value, startupStatus.value));

  async function initialize(): Promise<void> {
    try {
      const payload = await desktopBridge.initializeWindow();
      appMetadata.value = payload.app;
      settings.value = payload.settings;
      distribution.value = payload.distribution;
      currentWindow.value = payload.window;
      host.value = payload.host;
      applyLocale(resolveInitialLocale(payload.settings.locale));

      unlisteners.push(
        await desktopBridge.onGlobalSettingsChanged((value) => {
          settings.value = value;
          applyLocale(resolveInitialLocale(value.locale));
        }),
        await desktopBridge.onHostSnapshotChanged((value) => {
          host.value = value;
          syncWindowFromHost();
        }),
        await desktopBridge.onRuntimeDistributionChanged((value) => {
          distribution.value = value;
        }),
        await desktopBridge.onBuiltInRuntimeUpdated((urls) => {
          if (currentWindow.value && urls.includes(currentWindow.value.url)) {
            frameRevision.value += 1;
          }
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

  async function refreshCurrentWindow(): Promise<void> {
    const action = refreshAction.value;
    if (action === "refresh") {
      frameRevision.value += 1;
    } else if (action === "retry") {
      await retryStartup();
    }
  }

  async function saveGlobalSettings(patch: GlobalSettingsPatch): Promise<void> {
    error.value = null;
    try {
      settings.value = await desktopBridge.updateGlobalSettings(patch);
      applyLocale(resolveInitialLocale(settings.value.locale));
    } catch (cause) {
      error.value = cause as AppError;
    }
  }

  async function stopService(url: string): Promise<void> {
    error.value = null;
    try {
      host.value = await desktopBridge.stopService(url);
      syncWindowFromHost();
    } catch (cause) {
      error.value = cause as AppError;
    }
  }

  async function restartService(url: string): Promise<void> {
    error.value = null;
    try {
      host.value = await desktopBridge.restartService(url);
      syncWindowFromHost();
      if (currentWindow.value?.url === url) frameRevision.value += 1;
    } catch (cause) {
      error.value = cause as AppError;
    }
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
    distribution.value = result.distribution;
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
    appMetadata: readonly(appMetadata),
    settings: readonly(settings),
    distribution: readonly(distribution),
    currentWindow: readonly(currentWindow),
    host: readonly(host),
    startupStatus: readonly(startupStatus),
    error: readonly(error),
    startupFailures: readonly(startupFailures),
    settingsOpen,
    frameRevision: readonly(frameRevision),
    frameUrl,
    refreshAction,
    windowTitle,
    initialize,
    setTarget,
    retryStartup,
    refreshCurrentWindow,
    saveGlobalSettings,
    stopService,
    restartService,
  };
}
