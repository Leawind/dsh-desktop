import type {
  AppError,
  BootstrapPayload,
  GlobalSettings,
  GlobalSettingsPatch,
  HostSnapshot,
  RuntimeUpdateResult,
  RuntimeUpdateSnapshot,
  WindowSnapshot,
  WindowStartupResult,
} from "@/types/desktop";

export type UnlistenFn = () => void;

const heartbeatIntervalMillis = 250;

function normalizeError(error: unknown): AppError {
  if (typeof error === "object" && error !== null && "code" in error) {
    const candidate = error as Partial<AppError>;
    if (typeof candidate.code === "string") return candidate as AppError;
  }
  return {
    code: "app.error.unknown",
    technicalDetails: error instanceof Error ? error.message : String(error),
  };
}

function token(): string {
  const queryToken = new URLSearchParams(window.location.search).get("token");
  if (queryToken) return queryToken;
  const [, session, , sessionToken] = window.location.pathname.split("/");
  return session === "session" ? (sessionToken ?? "") : "";
}

function windowLabel(): string {
  const queryLabel = new URLSearchParams(window.location.search).get("window");
  if (queryLabel) return queryLabel;
  const [, session, label] = window.location.pathname.split("/");
  return session === "session" ? decodeURIComponent(label ?? "") : "";
}

async function command<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  let response: Response;
  const sessionToken = token();
  try {
    response = await fetch(`/api/command?window=${encodeURIComponent(windowLabel())}`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        ...(sessionToken ? { "X-DSH-Desktop-Token": sessionToken } : {}),
      },
      body: JSON.stringify({ name, args }),
    });
  } catch (error) {
    throw normalizeError(error);
  }
  let payload: { value?: T; error?: AppError };
  try {
    payload = (await response.json()) as { value?: T; error?: AppError };
  } catch (error) {
    throw normalizeError(error);
  }
  if (!response.ok || payload.error) throw normalizeError(payload.error);
  return payload.value as T;
}

function poll<T>(load: () => Promise<T>, listener: (value: T) => void): Promise<UnlistenFn> {
  let previous = "";
  const update = async () => {
    try {
      const value = await load();
      const next = JSON.stringify(value);
      if (next !== previous) {
        previous = next;
        listener(value);
      }
    } catch {
      // Commands surface errors to their caller; background refresh can retry.
    }
  };
  void update();
  const timer = window.setInterval(() => void update(), 2_000);
  return Promise.resolve(() => window.clearInterval(timer));
}

function heartbeat(): UnlistenFn {
  const timer = window.setInterval(
    () => void command<void>("heartbeat").catch(() => {}),
    heartbeatIntervalMillis,
  );
  return () => window.clearInterval(timer);
}

export const desktopBridge = {
  startWindowHeartbeat: (): UnlistenFn => heartbeat(),
  initializeWindow: (): Promise<BootstrapPayload> => command("initialize_window"),
  getHostSnapshot: (): Promise<HostSnapshot> => command("get_host_snapshot"),
  startWindow: (): Promise<WindowStartupResult> => command("start_window"),
  setWindowTarget: (url: string): Promise<WindowSnapshot> => command("set_window_target", { url }),
  stopService: (url: string): Promise<HostSnapshot> => command("stop_service", { url }),
  restartService: (url: string): Promise<HostSnapshot> => command("restart_service", { url }),
  checkBuiltInRuntimeUpdate: (): Promise<RuntimeUpdateSnapshot> =>
    command("check_built_in_runtime_update"),
  updateBuiltInRuntime: (): Promise<RuntimeUpdateResult> => command("update_built_in_runtime"),
  updateGlobalSettings: (patch: GlobalSettingsPatch): Promise<GlobalSettings> =>
    command("update_global_settings", { patch }),
  onHostSnapshotChanged: (listener: (snapshot: HostSnapshot) => void): Promise<UnlistenFn> =>
    poll(() => command<HostSnapshot>("get_host_snapshot"), listener),
  onBootstrapChanged: (listener: (payload: BootstrapPayload) => void): Promise<UnlistenFn> =>
    poll(() => command<BootstrapPayload>("initialize_window"), listener),
};
