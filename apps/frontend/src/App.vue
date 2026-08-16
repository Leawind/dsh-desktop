<script setup lang="ts">
import { computed, onMounted, watch } from "vue";
import { useI18n } from "vue-i18n";

import { UiButton, UiStatus } from "@dsh-desktop/ui";

import { desktopBridge } from "@/bridge/desktop";
import { useDesktopApp } from "@/composables/useDesktopApp";
import DshFrame from "@/features/dsh-frame/DshFrame.vue";
import SettingsOverlay from "@/features/settings/SettingsOverlay.vue";
import AppTitlebar from "@/features/titlebar/AppTitlebar.vue";

const { t } = useI18n();
const desktop = useDesktopApp();
const pageScaleStyle = computed(() => {
  const scale = desktop.settings.value.pageScalePercent / 100;
  return {
    width: `${100 / scale}vw`,
    height: `${100 / scale}vh`,
    transform: `scale(${scale})`,
  };
});

function errorMessage(): string {
  const error = desktop.error.value;
  if (!error) return t("app.error.unknown");
  return t(error.code, error.args ?? {});
}

function attemptName(type: string): string {
  return t(`settings.attempt.type.${type}`);
}

onMounted(() => {
  watch(
    desktop.windowTitle,
    (title) => {
      void desktopBridge.window.setTitle(title);
    },
    { immediate: true },
  );
  void desktop.initialize();
});
</script>

<template>
  <div class="app-shell" :style="pageScaleStyle">
    <AppTitlebar
      :refresh-action="desktop.refreshAction.value"
      :title="desktop.windowTitle.value"
      @settings="desktop.settingsOpen.value = true"
      @refresh="desktop.refreshCurrentWindow"
    />
    <main class="app-content">
      <DshFrame :url="desktop.frameUrl.value" :revision="desktop.frameRevision.value" />

      <section
        v-if="desktop.startupStatus.value === 'starting'"
        class="connection-state"
        aria-live="polite"
      >
        <UiStatus tone="info" animated>{{ t("app.initializing") }}</UiStatus>
      </section>

      <section v-else-if="desktop.error.value" class="connection-state connection-state--error">
        <h1>{{ t("app.error.title") }}</h1>
        <p>{{ errorMessage() }}</p>
        <details v-if="desktop.error.value.technicalDetails">
          <summary>{{ t("common.details") }}</summary>
          <pre>{{ desktop.error.value.technicalDetails }}</pre>
        </details>
        <ol v-if="desktop.startupFailures.value.length" class="connection-state__failures">
          <li v-for="(failure, index) in desktop.startupFailures.value" :key="index">
            <strong>{{ attemptName(failure.attempt.type) }}</strong>
            <span>{{ t(failure.error.code, failure.error.args ?? {}) }}</span>
          </li>
        </ol>
        <UiButton variant="primary" @click="desktop.retryStartup">
          {{ t("common.retry") }}
        </UiButton>
      </section>

      <SettingsOverlay
        v-if="desktop.settingsOpen.value && desktop.currentWindow.value"
        :current-url="desktop.currentWindow.value.url"
        :current-window-label="desktop.currentWindow.value.label"
        :app-metadata="desktop.appMetadata.value"
        :settings="desktop.settings.value"
        :host="desktop.host.value"
        :distribution="desktop.distribution.value"
        @close="desktop.settingsOpen.value = false"
        @set-target="desktop.setTarget"
        @save-settings="desktop.saveGlobalSettings"
        @stop-service="desktop.stopService"
        @restart-service="desktop.restartService"
      />
    </main>
  </div>
</template>

<style scoped>
.app-shell {
  --titlebar-height: 2.5rem;

  display: grid;
  width: 100vw;
  height: 100vh;
  grid-template-rows: var(--titlebar-height) minmax(0, 1fr);
  overflow: hidden;
  background: var(--color-background);
  transform-origin: top left;
}

.app-content {
  position: relative;
  min-width: 0;
  min-height: 0;
}

.connection-state {
  position: absolute;
  z-index: 10;
  inset: 0;
  display: grid;
  place-content: center;
  justify-items: center;
  gap: var(--space-4);
  padding: var(--space-6);
  background: var(--color-background);
}

.connection-state h1,
.connection-state p {
  margin: 0;
}

.connection-state p {
  color: var(--color-text-secondary);
}

.connection-state details {
  width: min(100%, 42rem);
  color: var(--color-text-secondary);
  font-size: var(--font-size-xs);
}

.connection-state pre {
  max-height: 12rem;
  overflow: auto;
  white-space: pre-wrap;
}

.connection-state__failures {
  display: grid;
  width: min(100%, 42rem);
  max-height: 14rem;
  gap: var(--space-2);
  margin: 0;
  padding-left: 1.5rem;
  overflow: auto;
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
}

.connection-state__failures li span {
  display: block;
}
</style>
