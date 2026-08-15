import { createI18n } from "vue-i18n";

import enUS from "./locales/en-US.json";
import zhCN from "./locales/zh-CN.json";

export const supportedLocales = ["zh-CN", "en-US"] as const;
export type AppLocale = (typeof supportedLocales)[number];

export const fallbackLocale: AppLocale = "zh-CN";

export const i18n = createI18n({
  legacy: false,
  locale: fallbackLocale,
  fallbackLocale,
  messages: {
    "zh-CN": zhCN,
    "en-US": enUS,
  },
});
