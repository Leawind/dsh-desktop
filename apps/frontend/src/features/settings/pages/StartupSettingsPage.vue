<script setup lang="ts">
import { Delete, Plus } from "@element-plus/icons-vue";
import { ElIcon, ElInputNumber, ElSelect } from "element-plus";
import { onBeforeUnmount, onMounted, ref } from "vue";

import { UiButton, UiInput } from "@dsh-desktop/ui";

import type { WindowStartupAttempt } from "@/types/desktop";

import { moveItem } from "./startupAttemptOrdering";

defineProps<{
  error: string;
  attemptOptions: { value: string; label: string }[];
}>();

const emit = defineEmits<{
  select: [];
}>();

const attempts = defineModel<WindowStartupAttempt[]>("attempts", { required: true });
const draggingAttemptKey = ref<string | null>(null);
const draggingAttemptOver = ref<{ key: string; half: "before" | "after" } | null>(null);
const attemptKeys = new WeakMap<WindowStartupAttempt, string>();
let nextAttemptKey = 0;
let draggingAttemptDropCommitted = false;

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

function rowHalf(event: DragEvent): "before" | "after" {
  const row = event.currentTarget as HTMLElement;
  const rect = row.getBoundingClientRect();
  return event.clientY < rect.top + rect.height / 2 ? "before" : "after";
}

function startDraggingAttempt(attempt: WindowStartupAttempt, event: DragEvent): void {
  const key = attemptKey(attempt);
  if (!event.dataTransfer) return;
  event.dataTransfer.effectAllowed = "move";
  event.dataTransfer.setData("text/plain", key);
  draggingAttemptDropCommitted = false;
  draggingAttemptKey.value = key;
  draggingAttemptOver.value = null;
}

function commitDraggingAttempt(over: { key: string; half: "before" | "after" }): void {
  if (draggingAttemptDropCommitted || !draggingAttemptKey.value) return;
  draggingAttemptDropCommitted = true;
  const sourceKey = draggingAttemptKey.value;
  draggingAttemptKey.value = null;
  draggingAttemptOver.value = null;

  const sourceIndex = attempts.value.findIndex((attempt) => attemptKey(attempt) === sourceKey);
  const targetIndex = attempts.value.findIndex((attempt) => attemptKey(attempt) === over.key);
  if (sourceIndex < 0 || targetIndex < 0) return;

  const anchorIndex = over.half === "before" ? targetIndex : targetIndex + 1;
  const insertIndex = sourceIndex < anchorIndex ? anchorIndex - 1 : anchorIndex;
  if (moveItem(attempts.value, sourceIndex, insertIndex)) emit("select");
}

function dragOverAttempt(event: DragEvent): void {
  if (!draggingAttemptKey.value) return;
  event.preventDefault();
  if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
  draggingAttemptOver.value = {
    key: attemptKeyFromElement(event.currentTarget as HTMLElement),
    half: rowHalf(event),
  };
}

function dropAttempt(event: DragEvent): void {
  if (!draggingAttemptKey.value) return;
  event.preventDefault();
  commitDraggingAttempt({
    key: attemptKeyFromElement(event.currentTarget as HTMLElement),
    half: rowHalf(event),
  });
}

function endDraggingAttempt(): void {
  if (draggingAttemptOver.value) commitDraggingAttempt(draggingAttemptOver.value);
  else {
    draggingAttemptKey.value = null;
    draggingAttemptOver.value = null;
  }
  draggingAttemptDropCommitted = false;
}

function attemptKeyFromElement(element: HTMLElement): string {
  return element.dataset.startupAttemptKey ?? "";
}

function acceptNativeDrag(event: DragEvent): void {
  if (!draggingAttemptKey.value) return;
  event.preventDefault();
  if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
}

function acceptNativeDrop(event: DragEvent): void {
  if (draggingAttemptKey.value) event.preventDefault();
}

onMounted(() => {
  document.addEventListener("dragover", acceptNativeDrag);
  document.addEventListener("drop", acceptNativeDrop);
});

onBeforeUnmount(() => {
  document.removeEventListener("dragover", acceptNativeDrag);
  document.removeEventListener("drop", acceptNativeDrop);
});
</script>

<template>
  <section
    id="settings-panel-startup"
    class="settings-page"
    role="tabpanel"
    aria-labelledby="settings-tab-startup"
  >
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
        :class="{
          'settings-page__attempt--dragging': draggingAttemptKey === attemptKey(attempt),
          'settings-page__attempt--drop-before':
            draggingAttemptOver?.key === attemptKey(attempt) &&
            draggingAttemptOver.half === 'before',
          'settings-page__attempt--drop-after':
            draggingAttemptOver?.key === attemptKey(attempt) &&
            draggingAttemptOver.half === 'after',
        }"
        :data-startup-attempt-key="attemptKey(attempt)"
        draggable="true"
        @dragstart="startDraggingAttempt(attempt, $event)"
        @dragend="endDraggingAttempt"
        @dragover.capture="dragOverAttempt"
        @drop.capture="dropAttempt"
      >
        <button
          class="settings-page__attempt-drag-handle"
          type="button"
          :aria-label="$t('settings.attempt.dragHandle')"
          :title="$t('settings.attempt.dragHandle')"
          tabindex="-1"
          @mousedown.stop
        >
          <span class="settings-page__attempt-drag-dots" aria-hidden="true">
            <span v-for="dot in 6" :key="dot" />
          </span>
        </button>
        <div class="settings-page__attempt-fields">
          <!-- @vue-ignore Element Plus 2.14.4 options typing is incompatible with TypeScript 6. -->
          <ElSelect
            :model-value="attempt.type"
            class="dsh-select"
            popper-class="dsh-select-popper"
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
  position: relative;
  display: grid;
  grid-template-columns: 1.5rem minmax(0, 1fr) 1.5rem;
  align-items: start;
  gap: var(--space-2);
  padding: var(--space-3);
  border: 0.0625rem solid var(--color-border);
  border-radius: var(--radius-control);
  background: var(--color-menu);
}

.settings-page__attempt--dragging {
  opacity: 0.5;
}

.settings-page__attempt--drop-before::before,
.settings-page__attempt--drop-after::after {
  position: absolute;
  z-index: 1;
  right: 0;
  left: 0;
  height: 0.125rem;
  border-radius: 999px;
  background: var(--color-accent);
  content: "";
  pointer-events: none;
}

.settings-page__attempt--drop-before::before {
  top: -0.3125rem;
}

.settings-page__attempt--drop-after::after {
  bottom: -0.3125rem;
}

.settings-page__attempt-drag-handle {
  display: grid;
  width: 1.5rem;
  height: 1.5rem;
  padding: 0;
  place-items: center;
  color: var(--color-text-secondary);
  border: 0;
  background: transparent;
  cursor: grab;
  touch-action: none;
  user-select: none;
}

.settings-page__attempt-drag-handle:active {
  cursor: grabbing;
}

.settings-page__attempt-drag-handle:focus-visible {
  outline: 0.125rem solid var(--color-focus);
  outline-offset: 0.125rem;
}

.settings-page__attempt-drag-dots {
  display: grid;
  grid-template-columns: repeat(2, 0.1875rem);
  gap: 0.1875rem;
}

.settings-page__attempt-drag-dots span {
  width: 0.1875rem;
  height: 0.1875rem;
  border-radius: 50%;
  background: currentColor;
}

.settings-page__attempt-remove svg {
  width: 1rem;
  height: 1rem;
}

.settings-page__attempt-remove {
  width: 1.5rem;
  height: 1.5rem;
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
