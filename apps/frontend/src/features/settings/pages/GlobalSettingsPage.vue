<script setup lang="ts">
import { Delete, Plus, Rank } from "@element-plus/icons-vue";
import { ElIcon } from "element-plus";
import { ref } from "vue";
import { ElButton, ElInputNumber } from "element-plus";

import { UiButton, UiInput, UiSelect, UiSettingRow } from "@dsh-desktop/ui";

import type {
  AppLocale,
  DistributionSnapshot,
  DshHome,
  DshSource,
  WindowStartupAttempt,
} from "@/types/desktop";

const props = defineProps<{
  distribution: DistributionSnapshot;
  error: string;
  localeOptions: { value: string; label: string }[];
  sourceOptions: { value: string; label: string; disabled?: boolean }[];
  homeOptions: { value: string; label: string }[];
  attemptOptions: { value: string; label: string }[];
}>();

const locale = defineModel<AppLocale>("locale", { required: true });
const pageScalePercent = defineModel<number>("pageScalePercent", { required: true });
const sourceType = defineModel<DshSource["type"]>("sourceType", { required: true });
const customExecutable = defineModel<string>("customExecutable", { required: true });
const homeType = defineModel<DshHome["type"]>("homeType", { required: true });
const customDshHome = defineModel<string>("customDshHome", { required: true });
const attempts = defineModel<WindowStartupAttempt[]>("attempts", { required: true });
const idleTimeoutMinutes = defineModel<number>("idleTimeoutMinutes", { required: true });
const draggingAttemptIndex = ref<number | null>(null);
const attemptKeys = new WeakMap<WindowStartupAttempt, string>();
let nextAttemptKey = 0;
const pageScaleFactor = 1.25;

function clampPageScale(value: number): number {
  return Math.min(200, Math.max(50, value));
}

function normalizePageScale(value: number): number {
  return Number.parseFloat(value.toPrecision(12));
}

function setPageScale(value: number | null | undefined): void {
  if (typeof value === "number" && Number.isFinite(value)) {
    pageScalePercent.value = clampPageScale(normalizePageScale(value));
  }
}

function adjustPageScale(direction: 1 | -1): void {
  setPageScale(pageScalePercent.value * pageScaleFactor ** direction);
}

function formatPageScale(value: string | number): string {
  const numericValue = Number(value);
  return Number.isFinite(numericValue) ? String(Math.round(numericValue)) : "";
}

function parsePageScale(value: string): string {
  return value;
}

function attemptKey(attempt: WindowStartupAttempt): string {
  let key = attemptKeys.get(attempt);
  if (!key) {
    key = String(nextAttemptKey++);
    attemptKeys.set(attempt, key);
  }
  return key;
}

function changeAttemptType(index: number, type: string): void {
  const host = "127.0.0.1";
  const replacements: Record<WindowStartupAttempt["type"], WindowStartupAttempt> = {
    "known-services": { type: "known-services" },
    "connect-fixed": { type: "connect-fixed", host, port: 3080 },
    "start-fixed": { type: "start-fixed", host, port: 3080 },
    "start-range": { type: "start-range", host, startPort: 3080, endPort: 3090 },
  };
  const replacement = replacements[type as WindowStartupAttempt["type"]];
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

function startDraggingAttempt(index: number, event: DragEvent): void {
  draggingAttemptIndex.value = index;
  event.dataTransfer?.setData("text/plain", String(index));
  if (event.dataTransfer) event.dataTransfer.effectAllowed = "move";
}

function moveDraggedAttempt(targetIndex: number): void {
  const sourceIndex = draggingAttemptIndex.value;
  if (sourceIndex === null || sourceIndex === targetIndex) return;
  const [attempt] = attempts.value.splice(sourceIndex, 1);
  if (!attempt) return;
  attempts.value.splice(targetIndex, 0, attempt);
  draggingAttemptIndex.value = targetIndex;
}

function stopDraggingAttempt(): void {
  draggingAttemptIndex.value = null;
}
</script>

<template>
  <section
    id="settings-panel-general"
    class="settings-page"
    role="tabpanel"
    aria-labelledby="settings-tab-general"
  >
    <UiSettingRow :label="$t('settings.language')">
      <UiSelect v-model="locale" variant="pill" :options="localeOptions" />
    </UiSettingRow>
    <UiSettingRow
      control-id="page-scale"
      :label="$t('settings.pageScale')"
      :hint="$t('settings.pageScaleHint')"
    >
      <div class="settings-page__page-scale-control">
        <ElButton
          :aria-label="$t('settings.decreasePageScale')"
          :disabled="pageScalePercent <= 50"
          @click="adjustPageScale(-1)"
        >
          −
        </ElButton>
        <ElInputNumber
          id="page-scale"
          :model-value="pageScalePercent"
          :min="50"
          :max="200"
          :formatter="formatPageScale"
          :parser="parsePageScale"
          :aria-label="$t('settings.pageScale')"
          :controls="false"
          :value-on-clear="50"
          @update:model-value="setPageScale"
        />
        <ElButton
          :aria-label="$t('settings.increasePageScale')"
          :disabled="pageScalePercent >= 200"
          @click="adjustPageScale(1)"
        >
          +
        </ElButton>
      </div>
    </UiSettingRow>
    <UiSettingRow :label="$t('settings.source.label')" :hint="$t('settings.source.hint')">
      <UiSelect v-model="sourceType" variant="pill" :options="sourceOptions" />
    </UiSettingRow>
    <UiSettingRow
      v-if="sourceType === 'custom'"
      control-id="dsh-executable"
      :label="$t('settings.executable')"
      :hint="$t('settings.executableHint')"
    >
      <div class="settings-page__wide-control">
        <UiInput id="dsh-executable" v-model="customExecutable" />
      </div>
    </UiSettingRow>
    <UiSettingRow :label="$t('settings.home.label')" :hint="$t('settings.home.hint')">
      <UiSelect v-model="homeType" variant="pill" :options="homeOptions" />
    </UiSettingRow>
    <UiSettingRow
      v-if="homeType === 'custom'"
      control-id="dsh-home"
      :label="$t('settings.home.path')"
      :hint="$t('settings.home.pathHint')"
    >
      <div class="settings-page__wide-control">
        <UiInput id="dsh-home" v-model="customDshHome" />
      </div>
    </UiSettingRow>
    <UiSettingRow
      control-id="idle-timeout"
      :label="$t('settings.idleTimeout')"
      :hint="$t('settings.idleTimeoutHint')"
    >
      <div class="settings-page__wide-control">
        <UiInput
          id="idle-timeout"
          :model-value="String(idleTimeoutMinutes)"
          type="number"
          min="0"
          :placeholder="$t('settings.idleTimeoutUnit')"
          @update:model-value="idleTimeoutMinutes = Number($event)"
        />
      </div>
    </UiSettingRow>
    <div class="settings-page__attempt-heading">
      <div>
        <h2>{{ $t("settings.attempt.label") }}</h2>
        <p>{{ $t("settings.attempt.hint") }}</p>
      </div>
    </div>
    <ol class="settings-page__attempts">
      <li
        v-for="(attempt, index) in attempts"
        :key="attemptKey(attempt)"
        class="settings-page__attempt"
        :class="{ 'settings-page__attempt--dragging': draggingAttemptIndex === index }"
        @dragover.prevent="moveDraggedAttempt(index)"
        @drop.prevent="stopDraggingAttempt"
      >
        <span
          class="settings-page__attempt-drag-handle"
          draggable="true"
          :aria-label="$t('settings.attempt.dragHandle')"
          :title="$t('settings.attempt.dragHandle')"
          @dragstart="startDraggingAttempt(index, $event)"
          @dragend="stopDraggingAttempt"
        >
          <ElIcon aria-hidden="true"><Rank /></ElIcon>
        </span>
        <div class="settings-page__attempt-fields">
          <UiSelect
            :model-value="attempt.type"
            :options="attemptOptions"
            @update:model-value="changeAttemptType(index, $event)"
          />
          <template v-if="attempt.type !== 'known-services'">
            <UiInput v-model="attempt.host" :placeholder="$t('settings.attempt.host')" />
            <UiInput
              v-if="attempt.type !== 'start-range'"
              :model-value="String(attempt.port)"
              type="number"
              :placeholder="$t('settings.attempt.port')"
              @update:model-value="setPort(attempt, 'port', $event)"
            />
            <template v-else>
              <UiInput
                :model-value="String(attempt.startPort)"
                type="number"
                :placeholder="$t('settings.attempt.startPort')"
                @update:model-value="setPort(attempt, 'startPort', $event)"
              />
              <UiInput
                :model-value="String(attempt.endPort)"
                type="number"
                :placeholder="$t('settings.attempt.endPort')"
                @update:model-value="setPort(attempt, 'endPort', $event)"
              />
            </template>
          </template>
        </div>
        <div class="settings-page__attempt-actions">
          <UiButton
            variant="ghost"
            size="small"
            class="settings-page__attempt-remove"
            :aria-label="$t('common.remove')"
            :title="$t('common.remove')"
            @click="attempts.splice(index, 1)"
          >
            <ElIcon aria-hidden="true"><Delete /></ElIcon>
          </UiButton>
        </div>
      </li>
    </ol>
    <UiButton class="settings-page__attempt-add" @click="addAttempt">
      <ElIcon aria-hidden="true"><Plus /></ElIcon>
      {{ $t("common.add") }}
    </UiButton>
    <p v-if="error" class="settings-page__error settings-page__error--block">{{ error }}</p>
  </section>
</template>

<style scoped>
.settings-page {
  width: 100%;
}

.settings-page :deep(.ui-setting-row:last-child) {
  border-bottom: 0;
}

.settings-page__wide-control {
  width: 280px;
}

.settings-page__page-scale-control {
  display: flex;
  width: 160px;
}

.settings-page__page-scale-control :deep(.el-button) {
  width: 40px;
  margin: 0;
  border-radius: 0;
}

.settings-page__page-scale-control :deep(.el-button:first-child) {
  border-radius: var(--radius-control) 0 0 var(--radius-control);
}

.settings-page__page-scale-control :deep(.el-button:last-child) {
  border-radius: 0 var(--radius-control) var(--radius-control) 0;
}

.settings-page__page-scale-control :deep(.el-input-number) {
  width: 80px;
}

.settings-page__page-scale-control :deep(.el-input__wrapper) {
  border-radius: 0;
}

.settings-page__attempt-heading {
  padding: var(--space-4) 0 var(--space-2);
}

.settings-page__attempt-heading h2,
.settings-page__attempt-heading p {
  margin: 0;
}

.settings-page__attempt-heading h2 {
  color: var(--color-text-primary);
  font-size: var(--font-size-sm);
  font-weight: 500;
}

.settings-page__attempt-heading p {
  margin-top: var(--space-1);
  color: var(--color-text-secondary);
  font-size: var(--font-size-xs);
}

.settings-page__attempts {
  display: grid;
  gap: var(--space-2);
  margin: 0;
  padding: 0;
  list-style: none;
}

.settings-page__attempt {
  display: grid;
  grid-template-columns: 24px minmax(0, 1fr) 24px;
  align-items: start;
  gap: var(--space-2);
  padding: var(--space-3);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-control);
  background: var(--color-input);
}

.settings-page__attempt--dragging {
  opacity: 0.5;
}

.settings-page__attempt-drag-handle {
  display: grid;
  width: 24px;
  height: 24px;
  place-items: center;
  color: var(--color-text-secondary);
  cursor: grab;
}

.settings-page__attempt-drag-handle:active {
  cursor: grabbing;
}

.settings-page__attempt-drag-handle svg,
.settings-page__attempt-remove svg {
  width: 16px;
  height: 16px;
  fill: currentColor;
}

.settings-page__attempt-remove {
  width: 24px;
  height: 24px;
  padding: 0;
}

.settings-page__attempt-remove svg {
  fill: none;
  stroke: currentColor;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.25;
}

.settings-page__attempt-fields {
  display: grid;
  min-width: 0;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--space-2);
}

.settings-page__attempt-fields > :first-child {
  grid-column: 1 / -1;
}

.settings-page__attempt-actions {
  display: flex;
  justify-content: flex-end;
}

.settings-page__attempt-add {
  width: 100%;
  margin-top: var(--space-2);
}

.settings-page__error {
  color: var(--color-danger);
  font-size: var(--font-size-xs);
  line-height: var(--line-height-xs);
}

.settings-page__error--block {
  display: block;
  margin-top: var(--space-2);
}

@media (max-width: 40rem) {
  .settings-page__wide-control {
    width: 100%;
  }
}
</style>
