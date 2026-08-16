import { createI18n } from "vue-i18n";

import enUS from "./locales/en-US.json";
import zhCN from "./locales/zh-CN.json";

export const supportedLocales = ["zh-CN", "en-US"] as const;
export type AppLocale = (typeof supportedLocales)[number];

export const fallbackLocale: AppLocale = "zh-CN";
export const localeMessages = {
  "zh-CN": zhCN,
  "en-US": enUS,
} as const;

export function resolveInitialLocale(
  saved: AppLocale | null,
  systemLocale = navigator.language,
): AppLocale {
  if (saved) return saved;
  if (supportedLocales.includes(systemLocale as AppLocale)) return systemLocale as AppLocale;
  if (systemLocale.toLowerCase().startsWith("zh")) return "zh-CN";
  if (systemLocale.toLowerCase().startsWith("en")) return "en-US";
  return fallbackLocale;
}

export const i18n = createI18n({
  legacy: false,
  locale: fallbackLocale,
  fallbackLocale,
  messages: localeMessages,
});

export function applyLocale(locale: AppLocale): void {
  i18n.global.locale.value = locale;
  document.documentElement.lang = locale;
}
