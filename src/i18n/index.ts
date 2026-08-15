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

export function resolveInitialLocale(saved: AppLocale | null): AppLocale {
  if (saved) return saved;
  const browserLocale = navigator.language;
  if (supportedLocales.includes(browserLocale as AppLocale)) return browserLocale as AppLocale;
  if (browserLocale.toLowerCase().startsWith("zh")) return "zh-CN";
  if (browserLocale.toLowerCase().startsWith("en")) return "en-US";
  return fallbackLocale;
}

export const i18n = createI18n({
  legacy: false,
  locale: fallbackLocale,
  fallbackLocale,
  messages: localeMessages,
});
