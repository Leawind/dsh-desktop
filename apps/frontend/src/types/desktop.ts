export type AppLocale = "zh-CN" | "en-US";

export type ServiceStatus = "unreachable" | "starting" | "running" | "failed";

export type EndpointOwnership = "external" | "managed";

export interface GlobalSettings {
  defaultDshPort: number;
  locale: AppLocale | null;
  dshExecutable: string | null;
}

export interface GlobalSettingsPatch extends GlobalSettings {}

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
