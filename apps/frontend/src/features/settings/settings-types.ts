export const settingsTabs = ["interface", "dsh", "startup", "runtime", "about"] as const;

export type SettingsTab = (typeof settingsTabs)[number];
