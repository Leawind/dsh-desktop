<script setup lang="ts">
import { Delete, Plus, Rank } from "@element-plus/icons-vue";
import { ElIcon, ElInputNumber } from "element-plus";
import { ref } from "vue";

import { UiButton, UiInput, UiSelect, UiSettingRow } from "@dsh-desktop/ui";

import type { WindowStartupAttempt } from "@/types/desktop";

defineProps<{
  error: string;
  attemptOptions: { value: string; label: string }[];
}>();

const emit = defineEmits<{
  select: [];
}>();

const attempts = defineModel<WindowStartupAttempt[]>("attempts", { required: true });
const idleTimeoutMinutes = defineModel<number>("idleTimeoutMinutes", { required: true });
const draggingAttemptIndex = ref<number | null>(null);
const attemptKeys = new WeakMap<WindowStartupAttempt, string>();
let nextAttemptKey = 0;

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
  value: number | null | undefined,
): void {
  if (typeof value !== "number" || !Number.isFinite(value)) return;
  const port = Math.trunc(value);
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
    id="settings-panel-startup"
    class="settings-page"
    role="tabpanel"
    aria-labelledby="settings-tab-startup"
  >
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
            @change="emit('select')"
          />
          <template v-if="attempt.type !== 'known-services'">
            <UiInput v-model="attempt.host" :placeholder="$t('settings.attempt.host')" />
            <ElInputNumber
              v-if="attempt.type !== 'start-range'"
              :model-value="attempt.port"
              :min="1"
              :max="65535"
              :precision="0"
              :aria-label="$t('settings.attempt.port')"
              @update:model-value="setPort(attempt, 'port', $event)"
            />
            <template v-else>
              <ElInputNumber
                :model-value="attempt.startPort"
                :min="1"
                :max="65535"
                :precision="0"
                :aria-label="$t('settings.attempt.startPort')"
                @update:model-value="setPort(attempt, 'startPort', $event)"
              />
              <ElInputNumber
                :model-value="attempt.endPort"
                :min="1"
                :max="65535"
                :precision="0"
                :aria-label="$t('settings.attempt.endPort')"
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

.settings-page__wide-control {
  width: 280px;
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

.settings-page__attempt-fields :deep(.el-input-number) {
  width: 100%;
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
