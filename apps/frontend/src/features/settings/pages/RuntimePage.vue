<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";

import { UiButton, UiSettingRow, UiStatus } from "@dsh-desktop/ui";

import { desktopBridge } from "@/bridge/desktop";
import type { HostSnapshot } from "@/types/desktop";

const props = defineProps<{
  currentWindowLabel: string;
  host: HostSnapshot;
}>();

const emit = defineEmits<{
  stopService: [url: string];
  restartService: [url: string];
}>();

const { t } = useI18n();
const knownEndpoints = computed(() => props.host.endpoints.filter((endpoint) => endpoint.known));

function endpointHint(endpoint: HostSnapshot["endpoints"][number]): string {
  const details = [
    t(`service.${endpoint.ownership}`),
    t("service.windows", endpoint.connectedWindows),
  ];
  if (endpoint.runtimeVersion) details.push(`DSH ${endpoint.runtimeVersion}`);
  if (endpoint.pid) details.push(`PID ${endpoint.pid}`);
  return details.join(" · ");
}
</script>

<template>
  <section
    id="settings-panel-runtime"
    class="settings-page"
    role="tabpanel"
    aria-labelledby="settings-tab-runtime"
  >
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
        <UiButton size="small" @click="desktopBridge.focusWindow(appWindow.label)">
          {{ $t("window.focus") }}
        </UiButton>
        <UiButton
          v-if="appWindow.label !== currentWindowLabel"
          variant="ghost"
          size="small"
          @click="desktopBridge.closeWindow(appWindow.label)"
        >
          {{ $t("common.close") }}
        </UiButton>
      </div>
    </UiSettingRow>

    <h2 class="settings-page__group-title">{{ $t("runtime.knownServices") }}</h2>
    <p v-if="knownEndpoints.length === 0" class="settings-page__empty">
      {{ $t("runtime.noKnownServices") }}
    </p>
    <template v-for="endpoint in knownEndpoints" :key="endpoint.url">
      <UiSettingRow :label="endpoint.url" :hint="endpointHint(endpoint)">
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

.settings-page__logs {
  margin: 0 var(--space-4) var(--space-3);
  color: var(--color-text-secondary);
  font-size: var(--font-size-xs);
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
