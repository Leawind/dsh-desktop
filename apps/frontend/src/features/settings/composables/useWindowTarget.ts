import { ref, toValue, watch, type MaybeRefOrGetter } from "vue";
import { useI18n } from "vue-i18n";

export function useWindowTarget(
  currentUrl: MaybeRefOrGetter<string>,
  onSetTarget: (url: string) => void,
) {
  const { t } = useI18n();
  const url = ref(toValue(currentUrl));
  const error = ref("");
  const targetApplyDelayMs = 500;
  let targetApplyTimer: ReturnType<typeof setTimeout> | undefined;
  let syncingTarget = false;

  watch(
    () => toValue(currentUrl),
    (value) => {
      syncingTarget = true;
      url.value = value;
      syncingTarget = false;
    },
  );

  watch(
    url,
    () => {
      if (!syncingTarget) scheduleTargetApply();
    },
    { flush: "sync" },
  );

  function validatedTargetUrl(): string | null {
    const value = url.value.trim();
    let parsed: URL;
    try {
      parsed = new URL(value);
    } catch {
      error.value = t("window.error.invalidUrl");
      return null;
    }
    if ((parsed.protocol !== "http:" && parsed.protocol !== "https:") || !parsed.hostname) {
      error.value = t("window.error.unsupportedUrl");
      return null;
    }
    if (parsed.username || parsed.password) {
      error.value = t("window.error.urlCredentials");
      return null;
    }
    error.value = "";
    return value;
  }

  function scheduleTargetApply(): void {
    if (targetApplyTimer !== undefined) clearTimeout(targetApplyTimer);
    targetApplyTimer = undefined;
    const target = validatedTargetUrl();
    if (!target) return;
    targetApplyTimer = setTimeout(() => {
      targetApplyTimer = undefined;
      onSetTarget(target);
    }, targetApplyDelayMs);
  }

  function flush(): void {
    if (targetApplyTimer === undefined) return;
    clearTimeout(targetApplyTimer);
    targetApplyTimer = undefined;
    const target = validatedTargetUrl();
    if (target) onSetTarget(target);
  }

  return { url, error, flush };
}
