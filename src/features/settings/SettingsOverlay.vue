<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";

import { desktopBridge } from "@/bridge/desktop";
import type { AppLocale, GlobalSettings, HostSnapshot } from "@/types/desktop";
import { UiButton, UiField, UiInput, UiSelect, UiStatus } from "@/ui";

const props = defineProps<{
  currentUrl: string;
  settings: GlobalSettings;
  host: HostSnapshot;
}>();

const emit = defineEmits<{
  close: [];
  setTarget: [url: string];
  saveSettings: [settings: GlobalSettings];
  reload: [];
}>();

const { t } = useI18n();
const url = ref(props.currentUrl);
const port = ref(String(props.settings.defaultDshPort));
const locale = ref<AppLocale>(props.settings.locale ?? "zh-CN");
const executable = ref(props.settings.dshExecutable ?? "");
const portError = ref("");

watch(
  () => props.currentUrl,
  (value) => {
    url.value = value;
  },
);

watch(
  () => props.settings,
  (value) => {
    port.value = String(value.defaultDshPort);
    locale.value = value.locale ?? "zh-CN";
    executable.value = value.dshExecutable ?? "";
  },
);

const localeOptions = computed(() => [
  { value: "zh-CN", label: t("locale.zh-CN") },
  { value: "en-US", label: t("locale.en-US") },
]);

function save(): void {
  const numericPort = Number(port.value);
  if (!Number.isInteger(numericPort) || numericPort < 1 || numericPort > 65535) {
    portError.value = t("settings.error.invalidPort");
    return;
  }
  portError.value = "";
  emit("saveSettings", {
    defaultDshPort: numericPort,
    locale: locale.value,
    dshExecutable: executable.value.trim() || null,
  });
}
</script>

<template>
  <section class="settings" role="dialog" aria-modal="true" :aria-label="t('settings.title')">
    <div class="settings__panel">
      <header class="settings__header">
        <h1>{{ t("settings.title") }}</h1>
        <UiButton variant="ghost" size="small" @click="$emit('close')">
          {{ t("common.close") }}
        </UiButton>
      </header>

      <div class="settings__content">
        <section class="settings__section">
          <h2>{{ t("window.current") }}</h2>
          <UiField input-id="current-url" :label="t('window.url')" :hint="t('window.urlHint')">
            <UiInput id="current-url" v-model="url" type="url" />
          </UiField>
          <div class="settings__actions">
            <UiButton variant="primary" @click="$emit('setTarget', url)">
              {{ t("common.save") }}
            </UiButton>
            <UiButton @click="$emit('reload')">{{ t("common.reload") }}</UiButton>
            <UiButton @click="desktopBridge.createWindow()">{{ t("window.new") }}</UiButton>
          </div>
        </section>

        <section class="settings__section">
          <h2>{{ t("settings.global") }}</h2>
          <UiField
            input-id="default-port"
            :label="t('settings.defaultPort')"
            :hint="t('settings.defaultPortHint')"
            :error="portError"
          >
            <UiInput id="default-port" v-model="port" type="number" />
          </UiField>
          <UiField input-id="locale" :label="t('settings.language')">
            <UiSelect id="locale" v-model="locale" :options="localeOptions" />
          </UiField>
          <UiField
            input-id="dsh-executable"
            :label="t('settings.executable')"
            :hint="t('settings.executableHint')"
          >
            <UiInput id="dsh-executable" v-model="executable" />
          </UiField>
          <div class="settings__actions">
            <UiButton variant="primary" @click="save">{{ t("common.save") }}</UiButton>
          </div>
        </section>

        <section class="settings__section">
          <h2>{{ t("service.section") }}</h2>
          <p v-if="host.endpoints.length === 0" class="settings__empty">
            {{ t("service.status.unreachable") }}
          </p>
          <article v-for="endpoint in host.endpoints" :key="endpoint.url" class="endpoint">
            <div class="endpoint__summary">
              <code>{{ endpoint.url }}</code>
              <UiStatus :tone="endpoint.status === 'running' ? 'success' : 'danger'">
                {{ t(`service.status.${endpoint.status}`) }}
              </UiStatus>
            </div>
            <div class="endpoint__metadata">
              <span>{{ t(`service.${endpoint.ownership}`) }}</span>
              <span>{{ t("service.windows", endpoint.connectedWindows) }}</span>
              <span v-if="endpoint.runtimeVersion">DSH {{ endpoint.runtimeVersion }}</span>
              <span v-if="endpoint.pid">PID {{ endpoint.pid }}</span>
            </div>
          </article>
        </section>
      </div>
    </div>
  </section>
</template>

<style scoped>
.settings {
  position: absolute;
  z-index: 20;
  inset: 0;
  display: grid;
  justify-items: start;
  overflow: auto;
  padding: var(--space-6);
  background: rgb(12 14 18 / 38%);
  backdrop-filter: blur(3px);
}

.settings__panel {
  width: min(100%, 42rem);
  min-height: 100%;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-panel);
  background: var(--color-surface);
  box-shadow: var(--shadow-panel);
}

.settings__header {
  position: sticky;
  z-index: 1;
  top: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-5) var(--space-6);
  border-bottom: 1px solid var(--color-border);
  border-radius: var(--radius-panel) var(--radius-panel) 0 0;
  background: var(--color-surface);
}

.settings__header h1,
.settings__section h2 {
  margin: 0;
  color: var(--color-text-primary);
}

.settings__header h1 {
  font-size: 1.125rem;
}

.settings__content,
.settings__section {
  display: grid;
}

.settings__content {
  gap: var(--space-8);
  padding: var(--space-6);
}

.settings__section {
  gap: var(--space-4);
}

.settings__section h2 {
  font-size: var(--font-size-sm);
}

.settings__actions,
.endpoint__metadata,
.endpoint__summary {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-3);
}

.settings__empty {
  margin: 0;
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
}

.endpoint {
  display: grid;
  gap: var(--space-2);
  padding: var(--space-4);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-control);
  background: var(--color-control);
}

.endpoint__summary {
  justify-content: space-between;
}

.endpoint__summary code {
  overflow: hidden;
  text-overflow: ellipsis;
}

.endpoint__metadata {
  color: var(--color-text-secondary);
  font-size: var(--font-size-xs);
}
</style>
