<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";

import { UiButton, UiSettingRow, UiStatus } from "@dsh-desktop/ui";

import { desktopBridge } from "@/bridge/desktop";
import type { DistributionSnapshot, HostSnapshot, RuntimeUpdateSnapshot } from "@/types/desktop";

const props = defineProps<{
  host: HostSnapshot;
  distribution: DistributionSnapshot;
  updateBuiltInRuntime: () => Promise<void>;
}>();

const emit = defineEmits<{
  stopService: [url: string];
  restartService: [url: string];
}>();

const { t } = useI18n();
const knownEndpoints = computed(() => props.host.endpoints.filter((endpoint) => endpoint.known));
const runtimeUpdate = ref<RuntimeUpdateSnapshot | null>(null);
const updateError = ref<string | null>(null);
const checkingForUpdate = ref(false);
const updating = ref(false);
const canUpdateBuiltIn = computed(() => props.distribution.variant === "bundled");

function endpointHint(endpoint: HostSnapshot["endpoints"][number]): string {
  const details = [
    t(`service.${endpoint.ownership}`),
    t("service.windows", endpoint.connectedWindows),
  ];
  if (endpoint.runtimeVersion) details.push(`DSH ${endpoint.runtimeVersion}`);
  if (endpoint.pid) details.push(`PID ${endpoint.pid}`);
  return details.join(" · ");
}

async function checkForUpdate(): Promise<void> {
  checkingForUpdate.value = true;
  updateError.value = null;
  try {
    runtimeUpdate.value = await desktopBridge.checkBuiltInRuntimeUpdate();
  } catch (error) {
    updateError.value = t(
      typeof error === "object" && error !== null && "code" in error
        ? String(error.code)
        : "app.error.unknown",
    );
  } finally {
    checkingForUpdate.value = false;
  }
}

async function updateRuntime(): Promise<void> {
  updating.value = true;
  updateError.value = null;
  try {
    await props.updateBuiltInRuntime();
    runtimeUpdate.value = null;
  } catch (error) {
    updateError.value = t(
      typeof error === "object" && error !== null && "code" in error
        ? String(error.code)
        : "app.error.unknown",
    );
  } finally {
    updating.value = false;
  }
}
</script>

<template>
  <section
    id="settings-panel-runtime"
    class="settings-page"
    role="tabpanel"
    aria-labelledby="settings-tab-runtime"
  >
    <h2 v-if="distribution.builtInRuntime" class="settings-page__group-title">
      {{ $t("runtime.builtInUpdate") }}
    </h2>
    <UiSettingRow
      v-if="distribution.builtInRuntime"
      :label="$t('runtime.currentVersion')"
      :hint="$t('runtime.updateHint')"
    >
      <div class="settings-page__inline-control">
        <UiStatus tone="info">DSH {{ distribution.builtInRuntime.dshVersion }}</UiStatus>
        <UiButton
          v-if="canUpdateBuiltIn"
          size="small"
          :disabled="checkingForUpdate || updating"
          @click="checkForUpdate"
        >
          {{ $t("runtime.checkForUpdates") }}
        </UiButton>
      </div>
    </UiSettingRow>
    <UiSettingRow
      v-if="runtimeUpdate?.candidateVersion"
      :label="$t('runtime.availableVersion')"
      :hint="
        runtimeUpdate.automaticRollbackSupported
          ? $t('runtime.rollbackReady')
          : $t('runtime.rollbackUnavailable')
      "
    >
      <div class="settings-page__inline-control">
        <UiStatus tone="info">DSH {{ runtimeUpdate.candidateVersion }}</UiStatus>
        <UiButton
          variant="primary"
          size="small"
          :disabled="updating || !runtimeUpdate.automaticRollbackSupported"
          @click="updateRuntime"
        >
          {{ $t("runtime.update") }}
        </UiButton>
      </div>
    </UiSettingRow>
    <p v-else-if="runtimeUpdate" class="settings-page__empty">
      {{ $t("runtime.upToDate") }}
    </p>
    <p v-if="updateError" class="settings-page__error">{{ updateError }}</p>

    <h2 class="settings-page__group-title">{{ $t("runtime.openWindows") }}</h2>
    <UiSettingRow
      v-for="appWindow in host.windows"
      :key="appWindow.label"
      :label="appWindow.label"
      :hint="appWindow.url || $t('window.noTarget')"
    >
      <div class="settings-page__inline-control">
        <UiStatus :tone="appWindow.status === 'running' ? 'success' : 'warning'">
          {{ $t(`service.status.${appWindow.status}`) }}
        </UiStatus>
      </div>
    </UiSettingRow>

    <h2 class="settings-page__group-title">{{ $t("runtime.knownServices") }}</h2>
    <p v-if="knownEndpoints.length === 0" class="settings-page__empty">
      {{ $t("runtime.noKnownServices") }}
    </p>
    <template v-for="endpoint in knownEndpoints" :key="endpoint.url">
      <UiSettingRow
        class="settings-page__dsh-service__row"
        :label="endpoint.url"
        :hint="endpointHint(endpoint)"
      >
        <div class="settings-page__inline-control">
          <UiStatus :tone="endpoint.status === 'running' ? 'success' : 'danger'">
            {{ $t(`service.status.${endpoint.status}`) }}
          </UiStatus>
          <UiButton
            v-if="endpoint.ownership === 'managed'"
            size="small"
            :disabled="!endpoint.canRestart"
            @click="$emit('restartService', endpoint.url)"
          >
            {{ $t("service.restart") }}
          </UiButton>
          <UiButton
            v-if="endpoint.ownership === 'managed'"
            variant="ghost"
            size="small"
            :disabled="!endpoint.canStop"
            @click="$emit('stopService', endpoint.url)"
          >
            {{ $t("service.stop") }}
          </UiButton>
        </div>
      </UiSettingRow>
      <details v-if="endpoint.logs.length" class="settings-page__logs">
        <summary>{{ $t("service.logs") }}</summary>
        <pre>{{ endpoint.logs.join("\n") }}</pre>
      </details>
    </template>
  </section>
</template>

<style scoped>
.settings-page {
  width: 100%;
}

.settings-page :deep(.ui-setting-row:last-child) {
  border-bottom: 0;
}

.settings-page__group-title {
  margin: 0;
  padding: var(--space-4) 0 var(--space-2);
  color: var(--color-text-primary);
  font-size: var(--font-size-sm);
  font-weight: 500;
}

.settings-page__dsh-service__row {
  border-bottom: 0;
}
.settings-page__inline-control {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: var(--space-2);
}

.settings-page__empty {
  margin: 0;
  padding: var(--space-4) 0;
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
  line-height: var(--line-height-sm);
}

.settings-page__error {
  margin: 0;
  padding: var(--space-3) 0;
  color: var(--color-danger);
  font-size: var(--font-size-sm);
}

.settings-page__logs {
  margin: 0 var(--space-4) var(--space-3);
  color: var(--color-text-secondary);
  font-size: var(--font-size-xs);
}
.settings-page__logs summary {
  cursor: pointer;
}
.settings-page__logs pre {
  max-height: 12rem;
  padding: var(--space-3);
  overflow: auto;
  border-radius: var(--radius-control);
  background: var(--color-background);
  white-space: pre-wrap;
}
</style>
