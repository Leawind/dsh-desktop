import { ref, toValue, watch, type MaybeRefOrGetter } from "vue";
import { useI18n } from "vue-i18n";

export function withDefaultTargetProtocol(input: string): string {
  const value = input.trim();
  return /^[a-z][a-z\d+.-]*:\/\//i.test(value) ? value : `http://${value}`;
}

export function useWindowTarget(
  currentUrl: MaybeRefOrGetter<string>,
  onSetTarget: (url: string) => void,
) {
  const { t } = useI18n();
  const url = ref(toValue(currentUrl));
  const error = ref("");
  let dirty = false;
  let syncingTarget = false;

  watch(
    () => toValue(currentUrl),
    (value) => {
      syncingTarget = true;
      url.value = value;
      syncingTarget = false;
      dirty = false;
    },
  );

  watch(
    url,
    () => {
      if (!syncingTarget) dirty = true;
    },
    { flush: "sync" },
  );

  function validatedTargetUrl(): string | null {
    const value = withDefaultTargetProtocol(url.value);
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

  function flush(): void {
    if (!dirty) return;
    const target = validatedTargetUrl();
    if (!target) return;
    onSetTarget(target);
    dirty = false;
  }

  return { url, error, flush };
}
