export type AppLocale = "zh-CN" | "en-US";

export type ServiceStatus = "unreachable" | "starting" | "running" | "failed";

export type EndpointOwnership = "external" | "managed";

export type DshSource =
  | { type: "none" }
  | { type: "built-in" }
  | { type: "system" }
  | { type: "custom"; executable: string };

export type WindowStartupAttempt =
  | { type: "known-services" }
  | { type: "connect-fixed"; host: string; port: number }
  | { type: "start-fixed"; host: string; port: number }
  | {
      type: "start-range";
      host: string;
      startPort: number;
      endPort: number;
    };

export interface GlobalSettings {
  locale: AppLocale | null;
  dshSource: DshSource;
  windowStartupAttempts: readonly WindowStartupAttempt[];
}

export interface GlobalSettingsPatch {
  locale: AppLocale | null;
  dshSource: DshSource;
  windowStartupAttempts: WindowStartupAttempt[];
}

export interface WindowSnapshot {
  label: string;
  url: string;
  status: ServiceStatus;
}

export interface EndpointSnapshot {
  url: string;
  status: ServiceStatus;
  ownership: EndpointOwnership;
  connectedWindows: number;
  pid: number | null;
  runtimeVersion: string | null;
  lastError: string | null;
  known: boolean;
}

export interface HostSnapshot {
  windows: readonly WindowSnapshot[];
  endpoints: readonly EndpointSnapshot[];
}

export interface BootstrapPayload {
  settings: GlobalSettings;
  window: WindowSnapshot;
  host: HostSnapshot;
}

export interface AppError {
  code: string;
  args?: Record<string, string | number>;
  technicalDetails?: string;
}

export interface StartupAttemptFailure {
  attempt: WindowStartupAttempt;
  error: AppError;
}

export interface WindowStartupResult {
  connected: boolean;
  window: WindowSnapshot;
  host: HostSnapshot;
  failures: StartupAttemptFailure[];
}
