<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";

import { UiButton, UiInput, UiSelect, UiSettingRow, UiStatus } from "@dsh-desktop/ui";

import { desktopBridge } from "@/bridge/desktop";
import type {
  AppLocale,
  DshSource,
  GlobalSettings,
  GlobalSettingsPatch,
  HostSnapshot,
  WindowStartupAttempt,
} from "@/types/desktop";

const props = defineProps<{
  currentUrl: string;
  settings: GlobalSettings;
  host: HostSnapshot;
}>();

const emit = defineEmits<{
  close: [];
  setTarget: [url: string];
  runStartup: [];
  saveSettings: [settings: GlobalSettingsPatch];
  reload: [];
}>();

const tabs = ["window", "general", "services"] as const;
type SettingsTab = (typeof tabs)[number];

const { t } = useI18n();
const activeTab = ref<SettingsTab>("window");
const url = ref(props.currentUrl);
const locale = ref<AppLocale>(props.settings.locale ?? "zh-CN");
const sourceType = ref<DshSource["type"]>(props.settings.dshSource.type);
const customExecutable = ref(
  props.settings.dshSource.type === "custom" ? props.settings.dshSource.executable : "",
);
const attempts = ref<WindowStartupAttempt[]>(cloneAttempts(props.settings.windowStartupAttempts));
const settingsError = ref("");

watch(
  () => props.currentUrl,
  (value) => {
    url.value = value;
  },
);

watch(
  () => props.settings,
  (value) => {
    locale.value = value.locale ?? "zh-CN";
    sourceType.value = value.dshSource.type;
    customExecutable.value = value.dshSource.type === "custom" ? value.dshSource.executable : "";
    attempts.value = cloneAttempts(value.windowStartupAttempts);
  },
);

const localeOptions = computed(() => [
  { value: "zh-CN", label: t("locale.zh-CN") },
  { value: "en-US", label: t("locale.en-US") },
]);

const sourceOptions = computed(() => [
  { value: "none", label: t("settings.source.type.none") },
  { value: "built-in", label: t("settings.source.type.built-in") },
  { value: "system", label: t("settings.source.type.system") },
  { value: "custom", label: t("settings.source.type.custom") },
]);

const attemptOptions = computed(() => [
  { value: "known-services", label: t("settings.attempt.type.known-services") },
  { value: "connect-fixed", label: t("settings.attempt.type.connect-fixed") },
  { value: "start-fixed", label: t("settings.attempt.type.start-fixed") },
  { value: "start-range", label: t("settings.attempt.type.start-range") },
]);
const knownEndpoints = computed(() => props.host.endpoints.filter((endpoint) => endpoint.known));

function cloneAttempts(value: readonly WindowStartupAttempt[]): WindowStartupAttempt[] {
  return value.map((attempt) => ({ ...attempt }));
}

function save(): void {
  if (sourceType.value === "custom" && !customExecutable.value.trim()) {
    settingsError.value = t("settings.error.emptyExecutable");
    return;
  }
  if (!attempts.value.every(validAttempt)) {
    settingsError.value = t("settings.error.invalidAttempt");
    return;
  }
  settingsError.value = "";
  const dshSource: DshSource =
    sourceType.value === "custom"
      ? { type: "custom", executable: customExecutable.value.trim() }
      : { type: sourceType.value };
  emit("saveSettings", {
    locale: locale.value,
    dshSource,
    windowStartupAttempts: cloneAttempts(attempts.value),
  });
}

function validPort(port: number): boolean {
  return Number.isInteger(port) && port >= 1 && port <= 65535;
}

function validAttempt(attempt: WindowStartupAttempt): boolean {
  if (attempt.type === "known-services") return true;
  if (!attempt.host.trim()) return false;
  if (attempt.type === "start-range") {
    return (
      validPort(attempt.startPort) &&
      validPort(attempt.endPort) &&
      attempt.startPort <= attempt.endPort
    );
  }
  return validPort(attempt.port);
}

function changeAttemptType(index: number, type: string): void {
  const host = "127.0.0.1";
  const replacements: Record<string, WindowStartupAttempt> = {
    "known-services": { type: "known-services" },
    "connect-fixed": { type: "connect-fixed", host, port: 3080 },
    "start-fixed": { type: "start-fixed", host, port: 3080 },
    "start-range": { type: "start-range", host, startPort: 3081, endPort: 3090 },
  };
  const replacement = replacements[type];
  if (replacement) attempts.value[index] = replacement;
}

function setPort(
  attempt: Exclude<WindowStartupAttempt, { type: "known-services" }>,
  field: "port" | "startPort" | "endPort",
  value: string,
): void {
  const port = Number(value);
  if (field === "port" && attempt.type !== "start-range") attempt.port = port;
  if (field === "startPort" && attempt.type === "start-range") attempt.startPort = port;
  if (field === "endPort" && attempt.type === "start-range") attempt.endPort = port;
}

function addAttempt(): void {
  attempts.value.push({ type: "connect-fixed", host: "127.0.0.1", port: 3080 });
}

function moveAttempt(index: number, offset: -1 | 1): void {
  const target = index + offset;
  if (target < 0 || target >= attempts.value.length) return;
  const current = attempts.value[index];
  const other = attempts.value[target];
  if (!current || !other) return;
  attempts.value[index] = other;
  attempts.value[target] = current;
}

function endpointHint(endpoint: HostSnapshot["endpoints"][number]): string {
  const details = [
    t(`service.${endpoint.ownership}`),
    t("service.windows", endpoint.connectedWindows),
  ];
  if (endpoint.runtimeVersion) details.push(`DSH ${endpoint.runtimeVersion}`);
  if (endpoint.pid) details.push(`PID ${endpoint.pid}`);
  return details.join(" · ");
}

function onTabKeydown(event: KeyboardEvent): void {
  let targetIndex: number | undefined;
  const currentIndex = tabs.indexOf(activeTab.value);

  if (event.key === "ArrowDown") targetIndex = (currentIndex + 1) % tabs.length;
  else if (event.key === "ArrowUp") targetIndex = (currentIndex - 1 + tabs.length) % tabs.length;
  else if (event.key === "Home") targetIndex = 0;
  else if (event.key === "End") targetIndex = tabs.length - 1;
  else return;

  event.preventDefault();
  const targetTab = tabs[targetIndex];
  if (!targetTab) return;
  activeTab.value = targetTab;
  const buttons = Array.from(
    (event.currentTarget as HTMLElement).parentElement?.querySelectorAll<HTMLButtonElement>(
      '[role="tab"]',
    ) ?? [],
  );
  buttons[targetIndex]?.focus();
}
</script>

<template>
  <section class="settings" role="presentation" @keydown.esc="$emit('close')">
    <div class="settings__mask" aria-hidden="true" @click="$emit('close')" />
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
            id="settings-tab-services"
            class="settings__tab"
            :class="{ 'settings__tab--active': activeTab === 'services' }"
            type="button"
            role="tab"
            :aria-selected="activeTab === 'services'"
            aria-controls="settings-panel-services"
            :tabindex="activeTab === 'services' ? 0 : -1"
            @click="activeTab = 'services'"
            @keydown="onTabKeydown"
          >
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <ellipse cx="8" cy="3.5" rx="5.25" ry="2" />
              <path
                d="M2.75 3.5v4c0 1.1 2.35 2 5.25 2s5.25-.9 5.25-2v-4M2.75 7.5v4c0 1.1 2.35 2 5.25 2s5.25-.9 5.25-2v-4"
              />
            </svg>
            <span>{{ t("service.section") }}</span>
          </button>
        </div>
      </nav>

      <div class="settings__content">
        <header class="settings__header">
          <div class="settings__header-actions">
            <UiButton v-if="activeTab === 'general'" variant="primary" size="small" @click="save">
              {{ t("common.save") }}
            </UiButton>
          </div>
          <button
            class="settings__close"
            type="button"
            :aria-label="t('common.close')"
            autofocus
            @click="$emit('close')"
          >
            <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true">
              <path d="m3 3 8 8M11 3l-8 8" />
            </svg>
          </button>
        </header>

        <div class="settings__options">
          <section
            v-show="activeTab === 'window'"
            id="settings-panel-window"
            class="settings__tabpanel"
            role="tabpanel"
            aria-labelledby="settings-tab-window"
          >
            <UiSettingRow
              control-id="current-url"
              :label="t('window.url')"
              :hint="t('window.urlHint')"
            >
              <div class="settings__inline-control settings__inline-control--wide">
                <UiInput id="current-url" v-model="url" type="url" />
                <UiButton variant="primary" size="small" @click="$emit('setTarget', url)">
                  {{ t("common.save") }}
                </UiButton>
              </div>
            </UiSettingRow>
            <UiSettingRow :label="t('window.actions')" :hint="t('window.actionsHint')">
              <div class="settings__inline-control">
                <UiButton size="small" @click="$emit('runStartup')">
                  {{ t("window.runStartup") }}
                </UiButton>
                <UiButton size="small" @click="$emit('reload')">
                  {{ t("common.reload") }}
                </UiButton>
                <UiButton size="small" @click="desktopBridge.createWindow()">
                  {{ t("window.new") }}
                </UiButton>
              </div>
            </UiSettingRow>
            <UiSettingRow
              v-for="endpoint in knownEndpoints"
              :key="endpoint.url"
              :label="endpoint.url"
              :hint="t('window.knownEndpointHint')"
            >
              <UiButton size="small" @click="$emit('setTarget', endpoint.url)">
                {{ t("common.connect") }}
              </UiButton>
            </UiSettingRow>
          </section>

          <section
            v-show="activeTab === 'general'"
            id="settings-panel-general"
            class="settings__tabpanel"
            role="tabpanel"
            aria-labelledby="settings-tab-general"
          >
            <UiSettingRow :label="t('settings.language')">
              <UiSelect v-model="locale" variant="pill" :options="localeOptions" />
            </UiSettingRow>
            <UiSettingRow :label="t('settings.source.label')" :hint="t('settings.source.hint')">
              <UiSelect v-model="sourceType" :options="sourceOptions" />
            </UiSettingRow>
            <UiSettingRow
              v-if="sourceType === 'custom'"
              control-id="dsh-executable"
              :label="t('settings.executable')"
              :hint="t('settings.executableHint')"
            >
              <div class="settings__wide-control">
                <UiInput id="dsh-executable" v-model="customExecutable" />
              </div>
            </UiSettingRow>
            <div class="settings__attempt-heading">
              <div>
                <h2>{{ t("settings.attempt.label") }}</h2>
                <p>{{ t("settings.attempt.hint") }}</p>
              </div>
              <UiButton size="small" @click="addAttempt">{{ t("common.add") }}</UiButton>
            </div>
            <ol class="settings__attempts">
              <li v-for="(attempt, index) in attempts" :key="index" class="settings__attempt">
                <span class="settings__attempt-index">{{ index + 1 }}</span>
                <div class="settings__attempt-fields">
                  <UiSelect
                    :model-value="attempt.type"
                    :options="attemptOptions"
                    @update:model-value="changeAttemptType(index, $event)"
                  />
                  <template v-if="attempt.type !== 'known-services'">
                    <UiInput v-model="attempt.host" :placeholder="t('settings.attempt.host')" />
                    <UiInput
                      v-if="attempt.type !== 'start-range'"
                      :model-value="String(attempt.port)"
                      type="number"
                      :placeholder="t('settings.attempt.port')"
                      @update:model-value="setPort(attempt, 'port', $event)"
                    />
                    <template v-else>
                      <UiInput
                        :model-value="String(attempt.startPort)"
                        type="number"
                        :placeholder="t('settings.attempt.startPort')"
                        @update:model-value="setPort(attempt, 'startPort', $event)"
                      />
                      <UiInput
                        :model-value="String(attempt.endPort)"
                        type="number"
                        :placeholder="t('settings.attempt.endPort')"
                        @update:model-value="setPort(attempt, 'endPort', $event)"
                      />
                    </template>
                  </template>
                </div>
                <div class="settings__attempt-actions">
                  <UiButton
                    variant="ghost"
                    size="small"
                    :disabled="index === 0"
                    @click="moveAttempt(index, -1)"
                  >
                    {{ t("common.moveUp") }}
                  </UiButton>
                  <UiButton
                    variant="ghost"
                    size="small"
                    :disabled="index === attempts.length - 1"
                    @click="moveAttempt(index, 1)"
                  >
                    {{ t("common.moveDown") }}
                  </UiButton>
                  <UiButton variant="ghost" size="small" @click="attempts.splice(index, 1)">
                    {{ t("common.remove") }}
                  </UiButton>
                </div>
              </li>
            </ol>
            <p v-if="settingsError" class="settings__error settings__error--block">
              {{ settingsError }}
            </p>
          </section>

          <section
            v-show="activeTab === 'services'"
            id="settings-panel-services"
            class="settings__tabpanel"
            role="tabpanel"
            aria-labelledby="settings-tab-services"
          >
            <p v-if="host.endpoints.length === 0" class="settings__empty">
              {{ t("service.status.unreachable") }}
            </p>
            <UiSettingRow
              v-for="endpoint in host.endpoints"
              :key="endpoint.url"
              :label="endpoint.url"
              :hint="endpointHint(endpoint)"
            >
              <UiStatus :tone="endpoint.status === 'running' ? 'success' : 'danger'">
                {{ t(`service.status.${endpoint.status}`) }}
              </UiStatus>
            </UiSettingRow>
          </section>
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

.settings__tab:hover {
  background: var(--color-nav-hover);
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
  justify-content: space-between;
  gap: var(--space-2);
  padding: 20px 14px 8px 10px;
}

.settings__header-actions {
  display: flex;
  min-width: 0;
  align-items: center;
  justify-content: flex-end;
  gap: var(--space-2);
  margin-left: auto;
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

.settings__tabpanel {
  width: 100%;
}

.settings__tabpanel :deep(.ui-setting-row:last-child) {
  border-bottom: 0;
}

.settings__inline-control {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: var(--space-2);
}

.settings__inline-control--wide {
  width: 320px;
}

.settings__inline-control--wide :deep(.ui-input) {
  width: auto;
  min-width: 0;
  flex: 1;
}

.settings__control-stack,
.settings__wide-control {
  width: 280px;
}

.settings__control-stack {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.settings__error {
  color: var(--color-danger);
  font-size: var(--font-size-xs);
  line-height: var(--line-height-xs);
}

.settings__empty {
  margin: 0;
  padding: var(--space-4) 0;
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
  line-height: var(--line-height-sm);
}

.settings__attempt-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-3);
  padding: var(--space-4) 0 var(--space-2);
}

.settings__attempt-heading h2,
.settings__attempt-heading p {
  margin: 0;
}

.settings__attempt-heading h2 {
  color: var(--color-text-primary);
  font-size: var(--font-size-sm);
  font-weight: 500;
}

.settings__attempt-heading p {
  margin-top: var(--space-1);
  color: var(--color-text-secondary);
  font-size: var(--font-size-xs);
}

.settings__attempts {
  display: grid;
  gap: var(--space-2);
  margin: 0;
  padding: 0;
  list-style: none;
}

.settings__attempt {
  display: grid;
  grid-template-columns: 24px minmax(0, 1fr) auto;
  align-items: start;
  gap: var(--space-2);
  padding: var(--space-3);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-control);
  background: var(--color-input);
}

.settings__attempt-index {
  display: grid;
  width: 24px;
  height: 24px;
  place-items: center;
  color: var(--color-text-secondary);
  font-size: var(--font-size-xs);
}

.settings__attempt-fields {
  display: grid;
  min-width: 0;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--space-2);
}

.settings__attempt-fields > :first-child {
  grid-column: 1 / -1;
}

.settings__attempt-actions {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.settings__error--block {
  display: block;
  margin-top: var(--space-2);
}

@media (max-width: 40rem) {
  .settings__panel {
    max-width: calc(100% - 24px);
  }

  .settings__nav {
    width: 148px;
    padding-inline: var(--space-2);
  }

  .settings__inline-control,
  .settings__inline-control--wide,
  .settings__control-stack,
  .settings__wide-control {
    width: 100%;
  }
}
</style>
