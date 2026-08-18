<script setup lang="ts">
import { onMounted } from "vue";
import { useI18n } from "vue-i18n";

import { UiButton, UiStatus } from "@dsh-desktop/ui";

import { useDesktopApp } from "@/composables/useDesktopApp";
import DshFrame from "@/features/dsh-frame/DshFrame.vue";
import SettingsOverlay from "@/features/settings/SettingsOverlay.vue";
import AppStatusbar from "@/features/statusbar/AppStatusbar.vue";

const { t } = useI18n();
const desktop = useDesktopApp();

function errorMessage(): string {
  const error = desktop.error.value;
  if (!error || typeof error.code !== "string") return t("app.error.unknown");
  return t(error.code, error.args ?? {});
}

function attemptName(type: string): string {
  return t(`settings.attempt.type.${type}`);
}

function statusMessage(): string | null {
  if (desktop.startupStatus.value === "starting") return t("app.initializing");
  if (desktop.error.value) return errorMessage();
  return null;
}

onMounted(() => {
  void desktop.initialize();
});
</script>

<template>
  <div class="app-shell">
    <main class="app-content">
      <DshFrame
        :url="desktop.frameUrl.value"
        :revision="desktop.frameRevision.value"
        :color-scheme="desktop.systemColorScheme.value"
      />

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
        :app-metadata="desktop.appMetadata.value"
        :settings="desktop.settings.value"
        :host="desktop.host.value"
        :distribution="desktop.distribution.value"
        :update-built-in-runtime="desktop.updateBuiltInRuntime"
        @close="desktop.settingsOpen.value = false"
        @save-settings="desktop.saveGlobalSettings"
        @stop-service="desktop.stopService"
        @restart-service="desktop.restartService"
      />
    </main>
    <AppStatusbar
      class="app-statusbar"
      :refresh-action="desktop.refreshAction.value"
      :status="desktop.startupStatus.value"
      :target-url="desktop.currentWindow.value?.url ?? ''"
      :message="statusMessage()"
      @settings="desktop.settingsOpen.value = true"
      @refresh="desktop.refreshCurrentWindow"
      @set-target="desktop.setTarget"
    />
  </div>
</template>

<style scoped>
.app-shell {
  display: flex;
  flex-flow: column nowrap;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: var(--color-background);
}

.app-content {
  flex: 1;
  position: relative;
  min-width: 0;
  min-height: 0;
}

.app-statusbar {
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
