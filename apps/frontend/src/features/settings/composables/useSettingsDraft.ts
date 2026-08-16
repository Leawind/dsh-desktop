import { computed, ref, toValue, watch, type MaybeRefOrGetter } from "vue";
import { useI18n } from "vue-i18n";

import { applyLocale, resolveInitialLocale } from "@/i18n";
import type {
  AppLocale,
  DistributionSnapshot,
  DshHome,
  DshSource,
  GlobalSettings,
  GlobalSettingsPatch,
  WindowStartupAttempt,
} from "@/types/desktop";

function cloneAttempts(value: readonly WindowStartupAttempt[]): WindowStartupAttempt[] {
  return value.map((attempt) => ({ ...attempt }));
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
  onSave: (patch: GlobalSettingsPatch) => void,
) {
  const { t } = useI18n();
  const initialSettings = toValue(settings);
  const locale = ref<AppLocale>(resolveInitialLocale(initialSettings.locale));
  const pageScalePercent = ref(initialSettings.pageScalePercent);
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
  let dirty = false;
  let syncingSettings = false;

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
    (value) => {
      syncingSettings = true;
      locale.value = resolveInitialLocale(value.locale);
      pageScalePercent.value = value.pageScalePercent;
      sourceType.value = value.dshSource.type;
      customExecutable.value = value.dshSource.type === "custom" ? value.dshSource.executable : "";
      npxVersion.value = value.dshSource.type === "npx" ? value.dshSource.version : "latest";
      homeType.value = value.dshHome.type;
      customDshHome.value = value.dshHome.type === "custom" ? value.dshHome.path : "";
      attempts.value = cloneAttempts(value.windowStartupAttempts);
      idleTimeoutMinutes.value = value.managedServiceIdleTimeoutSeconds / 60;
      syncingSettings = false;
      dirty = false;
    },
  );

  watch(
    [
      locale,
      pageScalePercent,
      sourceType,
      customExecutable,
      npxVersion,
      homeType,
      customDshHome,
      attempts,
      idleTimeoutMinutes,
    ],
    () => {
      if (syncingSettings) return;
      dirty = true;
    },
    { deep: true, flush: "sync" },
  );

  function buildPatch(): GlobalSettingsPatch | null {
    error.value = "";
    if (
      !Number.isFinite(pageScalePercent.value) ||
      pageScalePercent.value < 50 ||
      pageScalePercent.value > 200
    ) {
      error.value = t("settings.error.invalidPageScale");
      return null;
    }
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
    if (homeType.value === "custom" && !customDshHome.value.trim()) {
      error.value = t("settings.error.emptyDshHome");
      return null;
    }
    if (!attempts.value.every(validAttempt)) return null;
    if (
      !Number.isFinite(idleTimeoutMinutes.value) ||
      idleTimeoutMinutes.value < 0 ||
      idleTimeoutMinutes.value > 7 * 24 * 60
    ) {
      error.value = t("settings.error.invalidIdleTimeout");
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
    const dshHome: DshHome =
      homeType.value === "custom"
        ? { type: "custom", path: customDshHome.value.trim() }
        : { type: "environment" };
    return {
      locale: locale.value,
      pageScalePercent: pageScalePercent.value,
      dshSource,
      dshHome,
      windowStartupAttempts: cloneAttempts(attempts.value),
      managedServiceIdleTimeoutSeconds: Math.round(idleTimeoutMinutes.value * 60),
    };
  }

  function flush(): void {
    if (!dirty) return;
    const patch = buildPatch();
    if (!patch) return;
    applyLocale(locale.value);
    onSave(patch);
    dirty = false;
  }

  return {
    locale,
    pageScalePercent,
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
