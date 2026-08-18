<script setup lang="ts">
import { Close, Cpu, InfoFilled, Rank, Setting } from "@element-plus/icons-vue";
import { ElIcon } from "element-plus";
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
import AboutPage from "./pages/AboutPage.vue";
import DshSettingsPage from "./pages/DshSettingsPage.vue";
import InterfaceSettingsPage from "./pages/InterfaceSettingsPage.vue";
import RuntimePage from "./pages/RuntimePage.vue";
import StartupSettingsPage from "./pages/StartupSettingsPage.vue";
import { settingsTabs, type SettingsTab } from "./settings-types";

const props = defineProps<{
  appMetadata: AppMetadataSnapshot;
  settings: GlobalSettings;
  host: HostSnapshot;
  distribution: DistributionSnapshot;
  updateBuiltInRuntime: () => Promise<void>;
}>();

const emit = defineEmits<{
  close: [];
  saveSettings: [settings: GlobalSettingsPatch];
  stopService: [url: string];
  restartService: [url: string];
}>();

const { t } = useI18n();
const activeTab = ref<SettingsTab>("interface");
const {
  locale,
  sourceType,
  customExecutable,
  npxVersion,
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

function flushChanges(): void {
  flushSettings();
}

function close(): void {
  flushChanges();
  emit("close");
}

onBeforeUnmount(() => {
  flushChanges();
});
</script>

<template>
  <section class="settings" role="presentation" @focusout="flushChanges" @keydown.esc="close">
    <div class="settings__mask" aria-hidden="true" @click="close" />

    <div class="settings__panel" role="dialog" aria-modal="true" aria-labelledby="settings-title">
      <div class="settings__titlebar">
        <h1 id="settings-title" class="settings__title">{{ t("settings.title") }}</h1>
        <button
          class="settings__close"
          type="button"
          :aria-label="t('common.close')"
          autofocus
          @click="close"
        >
          <ElIcon aria-hidden="true"><Close /></ElIcon>
        </button>
      </div>

      <div class="settings__body">
        <nav class="settings__nav" :aria-label="t('settings.title')" role="tablist">
          <button
            id="settings-tab-interface"
            class="settings__tab"
            :class="{ 'settings__tab--active': activeTab === 'interface' }"
            type="button"
            role="tab"
            :aria-selected="activeTab === 'interface'"
            aria-controls="settings-panel-interface"
            :tabindex="activeTab === 'interface' ? 0 : -1"
            @click="activeTab = 'interface'"
            @keydown="onTabKeydown"
          >
            <ElIcon aria-hidden="true"><Setting /></ElIcon>
            <span>{{ t("settings.page.interface") }}</span>
          </button>
          <button
            id="settings-tab-dsh"
            class="settings__tab"
            :class="{ 'settings__tab--active': activeTab === 'dsh' }"
            type="button"
            role="tab"
            :aria-selected="activeTab === 'dsh'"
            aria-controls="settings-panel-dsh"
            :tabindex="activeTab === 'dsh' ? 0 : -1"
            @click="activeTab = 'dsh'"
            @keydown="onTabKeydown"
          >
            <ElIcon aria-hidden="true"><Cpu /></ElIcon>
            <span>{{ t("settings.page.dsh") }}</span>
          </button>
          <button
            id="settings-tab-startup"
            class="settings__tab"
            :class="{ 'settings__tab--active': activeTab === 'startup' }"
            type="button"
            role="tab"
            :aria-selected="activeTab === 'startup'"
            aria-controls="settings-panel-startup"
            :tabindex="activeTab === 'startup' ? 0 : -1"
            @click="activeTab = 'startup'"
            @keydown="onTabKeydown"
          >
            <ElIcon aria-hidden="true"><Rank /></ElIcon>
            <span>{{ t("settings.page.startup") }}</span>
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
            <ElIcon aria-hidden="true"><Cpu /></ElIcon>
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
            <ElIcon aria-hidden="true"><InfoFilled /></ElIcon>
            <span>{{ t("about.section") }}</span>
          </button>
        </nav>

        <div class="settings__content">
          <InterfaceSettingsPage
            v-show="activeTab === 'interface'"
            v-model:locale="locale"
            :locale-options="localeOptions"
            @select="flushSettings"
          />
          <DshSettingsPage
            v-show="activeTab === 'dsh'"
            v-model:source-type="sourceType"
            v-model:custom-executable="customExecutable"
            v-model:npx-version="npxVersion"
            v-model:home-type="homeType"
            v-model:custom-dsh-home="customDshHome"
            v-model:idle-timeout-minutes="idleTimeoutMinutes"
            :source-options="sourceOptions"
            :home-options="homeOptions"
            @select="flushSettings"
          />
          <StartupSettingsPage
            v-show="activeTab === 'startup'"
            v-model:attempts="attempts"
            :error="settingsError"
            :attempt-options="attemptOptions"
            @select="flushSettings"
          />
          <RuntimePage
            v-show="activeTab === 'runtime'"
            :host="host"
            :distribution="distribution"
            :dsh-source="settings.dshSource"
            :update-built-in-runtime="updateBuiltInRuntime"
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

      <div class="settings__background">
        <img src="/app-icon.png" />
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
  flex-flow: column nowrap;
  justify-content: space-between;
  align-items: stretch;
  width: 50rem;
  height: min(50rem, calc(100% - 3rem));
  max-width: calc(100% - 3rem);
  overflow: hidden;
  border-radius: 1.5rem;
  background: var(--color-surface-raised);
  box-shadow: var(--shadow-menu);
}

.settings__panel > .settings__background {
  pointer-events: none;

  display: flex;
  flex-flow: column nowrap;
  justify-content: flex-end;
  align-items: center;
  position: absolute;
  left: 0;
  top: 0;
  width: 100%;
  height: 100%;

  transition:
    opacity ease-in 60s,
    filter ease-in 60s;
}
body:hover .settings__panel > .settings__background {
  transition:
    opacity ease-in-out 140ms,
    filter ease-in-out 140ms;
  opacity: 0.1;
  filter: blur(0.5em);
}
.settings__panel > .settings__background > img {
  width: auto;
  height: 100%;
}

.settings__panel > .settings__titlebar {
  height: 3em;
  width: 100%;

  display: flex;
  flex-flow: row nowrap;
  justify-content: space-between;
  align-items: center;

  margin: 0;
  padding: 2em 1em;
}
.settings__title {
  margin: 0;
  padding: 0 0.75rem;
  color: var(--color-text-primary);
  font-size: var(--font-size-md);
  font-weight: 500;
  line-height: var(--line-height-md);
}
.settings__close {
  display: inline-flex;
  width: 1.75rem;
  height: 1.75rem;
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

.settings__panel > .settings__body {
  min-height: 0;
  flex: 1;

  display: flex;
  flex-flow: row nowrap;
  align-items: stretch;
}

.settings__panel > .settings__body > .settings__nav {
  display: flex;
  width: 11.75rem;
  padding: 0 0.75rem 0;

  display: flex;
  flex-flow: column nowrap;
  gap: var(--space-1);
}

.settings__panel > .settings__body > .settings__nav > .settings__tab {
  height: 2.8em;
  padding: 0.5625rem 0.75rem;

  display: flex;
  align-items: center;
  gap: var(--space-2);

  overflow: hidden;
  border: 0;
  border-radius: 0.75rem;
  background: none;

  color: var(--color-text-primary);
  font: inherit;
  font-size: var(--font-size-sm);
  line-height: var(--line-height-sm);

  cursor: pointer;
}
.settings__panel > .settings__body > .settings__nav > .settings__tab--active {
  background: var(--color-nav-active);
}

.settings__tab svg {
  width: 1.2em;
  height: 1.2em;
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
  min-height: 0;
  min-width: 0;
  height: auto;
  flex: 1;
  flex-direction: column;
  overflow-y: auto;
  padding: 0 1.5rem 1.5rem;
}

.settings__content::-webkit-scrollbar {
  width: 0.5rem;
  height: 0.5rem;
}
.settings__content::-webkit-scrollbar-corner {
  background: transparent;
}
.settings__content::-webkit-scrollbar-thumb {
  border-radius: 0.25rem;
  background: var(--color-scrollbar-thumb);
}
.settings__content::-webkit-scrollbar-thumb:hover {
  background: var(--color-scrollbar-thumb-hover);
}
.settings__content::-webkit-scrollbar-track {
  background: transparent;
}

@media (max-width: 40rem) {
  .settings__panel {
    max-width: calc(100% - 1.5rem);
  }

  .settings__panel > .settings__body > .settings__nav {
    width: 8rem;
    padding-inline: var(--space-2);
  }

  .settings__content {
    padding-inline: var(--space-4);
  }
}
</style>
