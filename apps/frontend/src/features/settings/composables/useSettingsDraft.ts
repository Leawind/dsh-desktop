import { computed, ref, toValue, watch, type MaybeRefOrGetter } from "vue";
import { useI18n } from "vue-i18n";

import { applyLocale, resolveInitialLocale } from "@/i18n";
import type {
  AppLocale,
  AppError,
  DistributionSnapshot,
  DshHome,
  DshSource,
  GlobalSettings,
  GlobalSettingsPatch,
  WindowStartupAttempt,
} from "@/types/desktop";

type SettingsGroup = "locale" | "source" | "home" | "attempts" | "idleTimeout";

function cloneAttempts(value: readonly WindowStartupAttempt[]): WindowStartupAttempt[] {
  return value.map((attempt) => ({ ...attempt }));
}

function cloneSettings(value: GlobalSettings): GlobalSettings {
  return {
    locale: value.locale,
    dshSource: { ...value.dshSource },
    dshHome: { ...value.dshHome },
    windowStartupAttempts: cloneAttempts(value.windowStartupAttempts),
    managedServiceIdleTimeoutSeconds: value.managedServiceIdleTimeoutSeconds,
  };
}

function equalValue(left: unknown, right: unknown): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}

function validPort(port: number): boolean {
  return Number.isInteger(port) && port >= 1 && port <= 65535;
}

function validAttempt(attempt: WindowStartupAttempt): boolean {
  if (attempt.type === "known-services") return true;
  if (!attempt.host.trim()) return false;
  if (attempt.type === "start-range") {
    return (
      validPort(attempt.startPort) &&
      validPort(attempt.endPort) &&
      attempt.startPort <= attempt.endPort
    );
  }
  return validPort(attempt.port);
}

export function useSettingsDraft(
  settings: MaybeRefOrGetter<GlobalSettings>,
  distribution: MaybeRefOrGetter<DistributionSnapshot>,
  onSave: (patch: GlobalSettingsPatch) => Promise<GlobalSettings>,
) {
  const { t } = useI18n();
  const initialSettings = toValue(settings);
  const locale = ref<AppLocale>(resolveInitialLocale(initialSettings.locale));
  const sourceType = ref<DshSource["type"]>(initialSettings.dshSource.type);
  const customExecutable = ref(
    initialSettings.dshSource.type === "custom" ? initialSettings.dshSource.executable : "",
  );
  const npxVersion = ref(
    initialSettings.dshSource.type === "npx" ? initialSettings.dshSource.version : "latest",
  );
  const homeType = ref<DshHome["type"]>(initialSettings.dshHome.type);
  const customDshHome = ref(
    initialSettings.dshHome.type === "custom" ? initialSettings.dshHome.path : "",
  );
  const attempts = ref<WindowStartupAttempt[]>(
    cloneAttempts(initialSettings.windowStartupAttempts),
  );
  const idleTimeoutMinutes = ref(initialSettings.managedServiceIdleTimeoutSeconds / 60);
  const error = ref("");
  let baseline = cloneSettings(initialSettings);
  let activeSave: Promise<void> | null = null;
  let saveQueued = false;

  const localeOptions = computed(() => [
    { value: "zh-CN", label: t("locale.zh-CN") },
    { value: "en-US", label: t("locale.en-US") },
  ]);
  const sourceOptions = computed(() => [
    { value: "none", label: t("settings.source.type.none") },
    {
      value: "built-in",
      label: t("settings.source.type.built-in"),
      disabled: toValue(distribution).variant !== "bundled",
    },
    { value: "system", label: t("settings.source.type.system") },
    { value: "custom", label: t("settings.source.type.custom") },
    { value: "npx", label: t("settings.source.type.npx") },
  ]);
  const homeOptions = computed(() => [
    { value: "environment", label: t("settings.home.type.environment") },
    { value: "custom", label: t("settings.home.type.custom") },
  ]);
  const attemptOptions = computed(() => [
    { value: "known-services", label: t("settings.attempt.type.known-services") },
    { value: "connect-fixed", label: t("settings.attempt.type.connect-fixed") },
    { value: "start-fixed", label: t("settings.attempt.type.start-fixed") },
    { value: "start-range", label: t("settings.attempt.type.start-range") },
  ]);

  watch(
    () => toValue(settings),
    (value) => syncFromSettings(value),
  );

  function draftLocale(): AppLocale | null {
    return locale.value === resolveInitialLocale(baseline.locale) ? baseline.locale : locale.value;
  }

  function draftDshSource(): DshSource {
    if (sourceType.value === "custom") {
      return { type: "custom", executable: customExecutable.value };
    }
    if (sourceType.value === "npx") {
      return { type: "npx", version: npxVersion.value };
    }
    return { type: sourceType.value };
  }

  function draftDshHome(): DshHome {
    return homeType.value === "custom"
      ? { type: "custom", path: customDshHome.value }
      : { type: "environment" };
  }

  function changedGroups(): Set<SettingsGroup> {
    const changed = new Set<SettingsGroup>();
    if (draftLocale() !== baseline.locale) changed.add("locale");
    if (!equalValue(draftDshSource(), baseline.dshSource)) changed.add("source");
    if (!equalValue(draftDshHome(), baseline.dshHome)) changed.add("home");
    if (!equalValue(attempts.value, baseline.windowStartupAttempts)) changed.add("attempts");
    if (Math.round(idleTimeoutMinutes.value * 60) !== baseline.managedServiceIdleTimeoutSeconds) {
      changed.add("idleTimeout");
    }
    return changed;
  }

  function syncFromSettings(value: GlobalSettings): void {
    const changed = changedGroups();
    if (!changed.has("locale")) locale.value = resolveInitialLocale(value.locale);
    if (!changed.has("source")) {
      sourceType.value = value.dshSource.type;
      customExecutable.value = value.dshSource.type === "custom" ? value.dshSource.executable : "";
      npxVersion.value = value.dshSource.type === "npx" ? value.dshSource.version : "latest";
    }
    if (!changed.has("home")) {
      homeType.value = value.dshHome.type;
      customDshHome.value = value.dshHome.type === "custom" ? value.dshHome.path : "";
    }
    if (!changed.has("attempts")) attempts.value = cloneAttempts(value.windowStartupAttempts);
    if (!changed.has("idleTimeout")) {
      idleTimeoutMinutes.value = value.managedServiceIdleTimeoutSeconds / 60;
    }
    baseline = cloneSettings(value);
    applyLocale(locale.value);
  }

  function buildPatch(): GlobalSettingsPatch | null {
    error.value = "";
    const changed = changedGroups();
    if (changed.size === 0) return null;
    const patch: GlobalSettingsPatch = {};

    if (changed.has("locale")) patch.locale = draftLocale();
    if (changed.has("source")) {
      if (sourceType.value === "built-in" && toValue(distribution).variant !== "bundled") {
        error.value = t("settings.error.unsupportedSource");
        return null;
      }
      if (sourceType.value === "custom" && !customExecutable.value.trim()) {
        error.value = t("settings.error.emptyExecutable");
        return null;
      }
      if (sourceType.value === "npx" && !validNpxVersion(npxVersion.value)) {
        error.value = t("settings.error.invalidNpxVersion");
        return null;
      }
      const dshSource: DshSource = (() => {
        if (sourceType.value === "custom") {
          return { type: "custom", executable: customExecutable.value.trim() };
        }
        if (sourceType.value === "npx") {
          return { type: "npx", version: npxVersion.value.trim() };
        }
        return { type: sourceType.value };
      })();
      patch.dshSource = dshSource;
    }
    if (changed.has("home")) {
      if (homeType.value === "custom" && !customDshHome.value.trim()) {
        error.value = t("settings.error.emptyDshHome");
        return null;
      }
      patch.dshHome =
        homeType.value === "custom"
          ? { type: "custom", path: customDshHome.value.trim() }
          : { type: "environment" };
    }
    if (changed.has("attempts")) {
      if (!attempts.value.every(validAttempt)) return null;
      patch.windowStartupAttempts = cloneAttempts(attempts.value);
    }
    if (changed.has("idleTimeout")) {
      if (
        !Number.isFinite(idleTimeoutMinutes.value) ||
        idleTimeoutMinutes.value < 0 ||
        idleTimeoutMinutes.value > 7 * 24 * 60
      ) {
        error.value = t("settings.error.invalidIdleTimeout");
        return null;
      }
      patch.managedServiceIdleTimeoutSeconds = Math.round(idleTimeoutMinutes.value * 60);
    }
    return patch;
  }

  function saveErrorMessage(cause: unknown): string {
    if (typeof cause === "object" && cause !== null && "code" in cause) {
      const appError = cause as Partial<AppError>;
      if (typeof appError.code === "string") return t(appError.code, appError.args ?? {});
    }
    return t("app.error.unknown");
  }

  async function flush(): Promise<void> {
    if (activeSave) {
      saveQueued = true;
      return activeSave;
    }
    const patch = buildPatch();
    if (!patch) return;
    if ("locale" in patch) applyLocale(locale.value);

    const save = (async () => {
      try {
        syncFromSettings(await onSave(patch));
        error.value = "";
      } catch (cause) {
        error.value = saveErrorMessage(cause);
      } finally {
        activeSave = null;
        if (saveQueued) {
          saveQueued = false;
          void flush();
        }
      }
    })();
    activeSave = save;
    return save;
  }

  return {
    locale,
    sourceType,
    customExecutable,
    npxVersion,
    homeType,
    customDshHome,
    attempts,
    idleTimeoutMinutes,
    error,
    localeOptions,
    sourceOptions,
    homeOptions,
    attemptOptions,
    flush,
  };
}

function validNpxVersion(value: string): boolean {
  return (
    value === "latest" || /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(value)
  );
}
