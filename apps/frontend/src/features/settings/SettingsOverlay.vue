<script setup lang="ts">
import { onBeforeUnmount, ref } from "vue";
import { useI18n } from "vue-i18n";

import type {
  AppMetadataSnapshot,
  DistributionSnapshot,
  GlobalSettings,
  GlobalSettingsPatch,
  HostSnapshot,
} from "@/types/desktop";

import { useSettingsDraft } from "./composables/useSettingsDraft";
import { useWindowTarget } from "./composables/useWindowTarget";
import AboutPage from "./pages/AboutPage.vue";
import CurrentWindowPage from "./pages/CurrentWindowPage.vue";
import GlobalSettingsPage from "./pages/GlobalSettingsPage.vue";
import RuntimePage from "./pages/RuntimePage.vue";
import { settingsTabs, type SettingsTab } from "./settings-types";

const props = defineProps<{
  currentUrl: string;
  currentWindowLabel: string;
  appMetadata: AppMetadataSnapshot;
  settings: GlobalSettings;
  host: HostSnapshot;
  distribution: DistributionSnapshot;
}>();

const emit = defineEmits<{
  close: [];
  setTarget: [url: string];
  saveSettings: [settings: GlobalSettingsPatch];
  stopService: [url: string];
  restartService: [url: string];
}>();

const { t } = useI18n();
const activeTab = ref<SettingsTab>("window");
const {
  url,
  error: urlError,
  flush: flushTarget,
} = useWindowTarget(
  () => props.currentUrl,
  (target) => emit("setTarget", target),
);
const {
  locale,
  sourceType,
  customExecutable,
  homeType,
  customDshHome,
  attempts,
  idleTimeoutMinutes,
  error: settingsError,
  localeOptions,
  sourceOptions,
  homeOptions,
  attemptOptions,
  flush: flushSettings,
} = useSettingsDraft(
  () => props.settings,
  () => props.distribution,
  (settings) => emit("saveSettings", settings),
);

function onTabKeydown(event: KeyboardEvent): void {
  let targetIndex: number | undefined;
  const currentIndex = settingsTabs.indexOf(activeTab.value);

  if (event.key === "ArrowDown") targetIndex = (currentIndex + 1) % settingsTabs.length;
  else if (event.key === "ArrowUp") {
    targetIndex = (currentIndex - 1 + settingsTabs.length) % settingsTabs.length;
  } else if (event.key === "Home") targetIndex = 0;
  else if (event.key === "End") targetIndex = settingsTabs.length - 1;
  else return;

  event.preventDefault();
  const targetTab = settingsTabs[targetIndex];
  if (!targetTab) return;
  activeTab.value = targetTab;
  const buttons = Array.from(
    (event.currentTarget as HTMLElement).parentElement?.querySelectorAll<HTMLButtonElement>(
      '[role="tab"]',
    ) ?? [],
  );
  buttons[targetIndex]?.focus();
}

onBeforeUnmount(() => {
  flushSettings();
  flushTarget();
});
</script>

<template>
  <section class="settings" role="presentation" @keydown.esc="emit('close')">
    <div class="settings__mask" aria-hidden="true" @click="emit('close')" />
    <div class="settings__panel" role="dialog" aria-modal="true" aria-labelledby="settings-title">
      <nav class="settings__nav" :aria-label="t('settings.title')">
        <h1 id="settings-title" class="settings__title">{{ t("settings.title") }}</h1>
        <div class="settings__tabs" role="tablist" :aria-label="t('settings.title')">
          <button
            id="settings-tab-window"
            class="settings__tab"
            :class="{ 'settings__tab--active': activeTab === 'window' }"
            type="button"
            role="tab"
            :aria-selected="activeTab === 'window'"
            aria-controls="settings-panel-window"
            :tabindex="activeTab === 'window' ? 0 : -1"
            @click="activeTab = 'window'"
            @keydown="onTabKeydown"
          >
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <rect x="2.25" y="3" width="11.5" height="9.5" rx="1.5" />
              <path d="M5.5 14h5" />
            </svg>
            <span>{{ t("window.current") }}</span>
          </button>
          <button
            id="settings-tab-general"
            class="settings__tab"
            :class="{ 'settings__tab--active': activeTab === 'general' }"
            type="button"
            role="tab"
            :aria-selected="activeTab === 'general'"
            aria-controls="settings-panel-general"
            :tabindex="activeTab === 'general' ? 0 : -1"
            @click="activeTab = 'general'"
            @keydown="onTabKeydown"
          >
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <circle cx="8" cy="8" r="2.25" />
              <path
                d="M8 1.75v1.5M8 12.75v1.5M1.75 8h1.5M12.75 8h1.5M3.58 3.58l1.06 1.06M11.36 11.36l1.06 1.06M12.42 3.58l-1.06 1.06M4.64 11.36l-1.06 1.06"
              />
            </svg>
            <span>{{ t("settings.global") }}</span>
          </button>
          <button
            id="settings-tab-runtime"
            class="settings__tab"
            :class="{ 'settings__tab--active': activeTab === 'runtime' }"
            type="button"
            role="tab"
            :aria-selected="activeTab === 'runtime'"
            aria-controls="settings-panel-runtime"
            :tabindex="activeTab === 'runtime' ? 0 : -1"
            @click="activeTab = 'runtime'"
            @keydown="onTabKeydown"
          >
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <path d="M3 3.5h10v9H3zM5.25 1.75h5.5M5.25 14.25h5.5" />
            </svg>
            <span>{{ t("runtime.section") }}</span>
          </button>
          <button
            id="settings-tab-about"
            class="settings__tab"
            :class="{ 'settings__tab--active': activeTab === 'about' }"
            type="button"
            role="tab"
            :aria-selected="activeTab === 'about'"
            aria-controls="settings-panel-about"
            :tabindex="activeTab === 'about' ? 0 : -1"
            @click="activeTab = 'about'"
            @keydown="onTabKeydown"
          >
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <circle cx="8" cy="8" r="6" />
              <path d="M8 7v4M8 4.5v.5" />
            </svg>
            <span>{{ t("about.section") }}</span>
          </button>
        </div>
      </nav>

      <div class="settings__content">
        <header class="settings__header">
          <button
            class="settings__close"
            type="button"
            :aria-label="t('common.close')"
            autofocus
            @click="emit('close')"
          >
            <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true">
              <path d="m3 3 8 8M11 3l-8 8" />
            </svg>
          </button>
        </header>

        <div class="settings__options">
          <CurrentWindowPage v-show="activeTab === 'window'" v-model:url="url" :error="urlError" />
          <GlobalSettingsPage
            v-show="activeTab === 'general'"
            v-model:locale="locale"
            v-model:source-type="sourceType"
            v-model:custom-executable="customExecutable"
            v-model:home-type="homeType"
            v-model:custom-dsh-home="customDshHome"
            v-model:attempts="attempts"
            v-model:idle-timeout-minutes="idleTimeoutMinutes"
            :distribution="distribution"
            :error="settingsError"
            :locale-options="localeOptions"
            :source-options="sourceOptions"
            :home-options="homeOptions"
            :attempt-options="attemptOptions"
          />
          <RuntimePage
            v-show="activeTab === 'runtime'"
            :current-window-label="currentWindowLabel"
            :host="host"
            @stop-service="emit('stopService', $event)"
            @restart-service="emit('restartService', $event)"
          />
          <AboutPage
            v-show="activeTab === 'about'"
            :app-metadata="appMetadata"
            :distribution="distribution"
          />
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.settings {
  position: absolute;
  z-index: 20;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}

.settings__mask {
  position: absolute;
  inset: 0;
  background: var(--color-mask);
  backdrop-filter: var(--backdrop-mask);
}

.settings__panel {
  position: relative;
  z-index: 1;
  display: flex;
  width: 800px;
  height: min(800px, calc(100% - 48px));
  max-width: calc(100% - 48px);
  overflow: hidden;
  border-radius: 24px;
  background: var(--color-surface-raised);
  box-shadow: var(--shadow-menu);
}

.settings__nav {
  display: flex;
  width: 188px;
  flex: none;
  flex-direction: column;
  gap: 18px;
  padding: 22px 12px 0;
}

.settings__title {
  margin: 0;
  padding: 0 12px;
  color: var(--color-text-primary);
  font-size: var(--font-size-md);
  font-weight: 500;
  line-height: var(--line-height-md);
}

.settings__tabs {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.settings__tab {
  display: flex;
  height: 40px;
  align-items: center;
  gap: var(--space-2);
  padding: 9px 16px 9px 12px;
  overflow: hidden;
  color: var(--color-text-primary);
  border: 0;
  border-radius: 12px;
  background: transparent;
  font: inherit;
  font-size: var(--font-size-sm);
  line-height: var(--line-height-sm);
  text-align: left;
  cursor: pointer;
}

.settings__tab--active {
  background: var(--color-nav-active);
}

.settings__tab svg {
  width: 16px;
  height: 16px;
  flex: none;
  fill: none;
  stroke: currentColor;
  stroke-width: 1.25;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.settings__tab span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.settings__content {
  display: flex;
  min-width: 0;
  flex: 1;
  flex-direction: column;
}

.settings__header {
  display: flex;
  height: 54px;
  flex: none;
  align-items: flex-start;
  justify-content: flex-end;
  padding: 20px 14px 8px 10px;
}

.settings__close {
  display: inline-flex;
  width: 28px;
  height: 28px;
  flex: none;
  align-items: center;
  justify-content: center;
  padding: 0;
  color: var(--color-text-primary);
  border: 0;
  border-radius: 50%;
  background: transparent;
  cursor: pointer;
}

.settings__close:hover {
  background: var(--color-interactive-hover);
}

.settings__close svg {
  fill: none;
  stroke: currentColor;
  stroke-width: 1.25;
  stroke-linecap: round;
}

.settings__options {
  min-height: 0;
  flex: 1;
  overflow-y: auto;
  padding: 0 24px 24px;
}

@media (max-width: 40rem) {
  .settings__panel {
    max-width: calc(100% - 24px);
  }

  .settings__nav {
    width: 148px;
    padding-inline: var(--space-2);
  }
}
</style>
