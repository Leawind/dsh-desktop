export type AppLocale = "zh-CN" | "en-US";

export type ServiceStatus =
  "unreachable" | "starting" | "stopping" | "restarting" | "updating" | "running" | "failed";

export type EndpointOwnership = "external" | "managed";

export type DistributionVariant = "bundled" | "slim";

export type SystemColorScheme = "light" | "dark";

export interface BundledRuntimeSnapshot {
  runtimeId: string;
  nodeVersion: string;
  dshVersion: string;
  pnpmVersion: string;
  installed: boolean;
}

export interface DistributionSnapshot {
  variant: DistributionVariant;
  builtInRuntime: BundledRuntimeSnapshot | null;
}

export interface AppMetadataSnapshot {
  name: string;
  version: string;
  identifier: string;
}

export type DshSource =
  | { type: "none" }
  | { type: "built-in" }
  | { type: "system" }
  | { type: "custom"; executable: string }
  | { type: "npx"; version: string };

export type DshHome = { type: "environment" } | { type: "custom"; path: string };

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
  dshHome: DshHome;
  windowStartupAttempts: readonly WindowStartupAttempt[];
  managedServiceIdleTimeoutSeconds: number;
}

export interface GlobalSettingsPatch {
  locale: AppLocale | null;
  dshSource: DshSource;
  dshHome: DshHome;
  windowStartupAttempts: WindowStartupAttempt[];
  managedServiceIdleTimeoutSeconds: number;
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
  canStop: boolean;
  canRestart: boolean;
  logs: readonly string[];
}

export interface HostSnapshot {
  windows: readonly WindowSnapshot[];
  endpoints: readonly EndpointSnapshot[];
}

export interface BootstrapPayload {
  app: AppMetadataSnapshot;
  settings: GlobalSettings;
  distribution: DistributionSnapshot;
  window: WindowSnapshot;
  host: HostSnapshot;
  systemColorScheme: SystemColorScheme | null;
}

export interface RuntimeUpdateSnapshot {
  candidateVersion: string | null;
}

export interface RuntimeUpdateResult {
  distribution: DistributionSnapshot;
  host: HostSnapshot;
  updatedUrls: readonly string[];
}

export interface AppUpdateCandidate {
  version: string;
  notes: string | null;
}

export interface AppUpdateSnapshot {
  currentVersion: string;
  candidate: AppUpdateCandidate | null;
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
  distribution: DistributionSnapshot;
  window: WindowSnapshot;
  host: HostSnapshot;
  failures: StartupAttemptFailure[];
}
