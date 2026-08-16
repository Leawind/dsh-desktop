export const settingsTabs = ["window", "general", "runtime", "about"] as const;

export type SettingsTab = (typeof settingsTabs)[number];
