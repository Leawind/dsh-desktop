<script setup lang="ts">
import { Close, FullScreen, Minus, RefreshRight, Setting } from "@element-plus/icons-vue";
import { computed, nextTick, onBeforeUnmount, ref } from "vue";
import { useI18n } from "vue-i18n";

import { UiButton, UiInput } from "@dsh-desktop/ui";

import { desktopBridge } from "@/bridge/desktop";
import { useWindowTarget } from "@/composables/useWindowTarget";

const props = defineProps<{
  refreshAction: "refresh" | "retry" | null;
  targetUrl: string;
}>();

const emit = defineEmits<{
  settings: [];
  refresh: [];
  setTarget: [url: string];
}>();

const { t } = useI18n();
const editing = ref(false);
const targetInput = ref<{ focus: () => void } | null>(null);
let targetGesture: { x: number; y: number; dragging: boolean } | undefined;
let suppressTargetClick = false;
const {
  url,
  error: urlError,
  flush: flushTarget,
} = useWindowTarget(
  () => props.targetUrl,
  (target) => emit("setTarget", target),
);

const targetLabel = computed(() => {
  if (!props.targetUrl) return t("window.noTarget");
  try {
    const endpoint = new URL(props.targetUrl);
    const port = endpoint.port || (endpoint.protocol === "http:" ? "80" : "443");
    return `${endpoint.hostname}:${port}`;
  } catch {
    return props.targetUrl;
  }
});

function beginEditing(): void {
  if (!props.targetUrl) return;
  editing.value = true;
  void nextTick(() => targetInput.value?.focus());
}

function finishEditing(): void {
  flushTarget();
  if (!urlError.value) editing.value = false;
}

function startTargetGesture(event: MouseEvent): void {
  finishTargetGesture();
  targetGesture = {
    x: event.clientX,
    y: event.clientY,
    dragging: false,
  };
  suppressTargetClick = false;
  document.addEventListener("mousemove", continueTargetGesture);
  document.addEventListener("mouseup", finishTargetGesture, { once: true });
}

function continueTargetGesture(event: MouseEvent): void {
  if (!targetGesture || targetGesture.dragging) return;
  if (Math.hypot(event.clientX - targetGesture.x, event.clientY - targetGesture.y) < 4) return;
  targetGesture.dragging = true;
  suppressTargetClick = true;
  void desktopBridge.window.startDragging();
}

function finishTargetGesture(): void {
  document.removeEventListener("mousemove", continueTargetGesture);
  document.removeEventListener("mouseup", finishTargetGesture);
  targetGesture = undefined;
}

function onTargetClick(): void {
  if (suppressTargetClick) {
    suppressTargetClick = false;
    return;
  }
  beginEditing();
}

onBeforeUnmount(finishTargetGesture);
</script>

<template>
  <header
    class="titlebar"
    @dblclick="desktopBridge.window.toggleMaximize()"
    @mousedown.left="desktopBridge.window.startDragging()"
  >
    <div class="titlebar__leading" @dblclick.stop>
      <img class="titlebar__app-icon" src="/app-icon.png" alt="" aria-hidden="true" />
      <UiButton
        variant="ghost"
        size="small"
        class="titlebar__button"
        :aria-label="t('window.settings')"
        @click.stop="$emit('settings')"
        @mousedown.stop
      >
        <Setting class="titlebar__icon" aria-hidden="true" />
      </UiButton>
      <UiButton
        variant="ghost"
        size="small"
        class="titlebar__button"
        :disabled="refreshAction === null"
        :aria-label="t(refreshAction === 'retry' ? 'common.retry' : 'window.refresh')"
        :title="t(refreshAction === 'retry' ? 'common.retry' : 'window.refresh')"
        @click.stop="$emit('refresh')"
        @mousedown.stop
      >
        <RefreshRight class="titlebar__icon" aria-hidden="true" />
      </UiButton>
    </div>
    <div
      class="titlebar__target"
      @dblclick.stop
      @mousedown.left="desktopBridge.window.startDragging()"
    >
      <button
        v-if="!editing"
        class="titlebar__target-button"
        type="button"
        :disabled="!targetUrl"
        :aria-label="t('window.url')"
        @mousedown.left.stop="startTargetGesture"
        @click="onTargetClick"
        @keydown.enter.prevent="beginEditing"
        @keydown.space.prevent="beginEditing"
      >
        {{ targetLabel }}
      </button>
      <UiInput
        v-else
        ref="targetInput"
        v-model="url"
        class="titlebar__target-input"
        type="url"
        content-sized
        :disabled="!targetUrl"
        :placeholder="t('window.noTarget')"
        :aria-label="t('window.url')"
        :aria-invalid="Boolean(urlError)"
        :title="urlError || undefined"
        @mousedown.stop
        @blur="finishEditing"
        @keydown.enter.prevent="finishEditing"
      />
    </div>
    <div class="titlebar__controls" @dblclick.stop>
      <UiButton
        variant="ghost"
        size="small"
        class="titlebar__button"
        :aria-label="t('window.minimize')"
        @click.stop="desktopBridge.window.minimize()"
        @mousedown.stop
      >
        <Minus class="titlebar__icon" aria-hidden="true" />
      </UiButton>
      <UiButton
        variant="ghost"
        size="small"
        class="titlebar__button"
        :aria-label="t('window.maximize')"
        @click.stop="desktopBridge.window.toggleMaximize()"
        @mousedown.stop
      >
        <FullScreen class="titlebar__icon" aria-hidden="true" />
      </UiButton>
      <UiButton
        variant="ghost"
        size="small"
        class="titlebar__button titlebar__button--close"
        :aria-label="t('window.close')"
        @click.stop="desktopBridge.window.close()"
        @mousedown.stop
      >
        <Close class="titlebar__icon" aria-hidden="true" />
      </UiButton>
    </div>
  </header>
</template>

<style scoped>
.titlebar {
  position: relative;
  z-index: 30;
  height: var(--titlebar-height);
  padding: 0;
  margin: 0;
  display: flex;
  flex-flow: row nowrap;
  justify-content: space-between;
  align-items: stretch;
  color: var(--color-text-secondary);
  border-bottom: 1px solid var(--color-border);
  background: var(--color-surface);
  user-select: none;
}

.titlebar__leading,
.titlebar__controls {
  flex-grow: 0;
  display: flex;
  align-items: center;
}

.titlebar__controls {
  justify-content: flex-end;
}

.titlebar__app-icon {
  height: var(--titlebar-height);
  width: auto;
  margin: 0 0.625rem;
  object-fit: contain;
  pointer-events: none;
}

.titlebar__target {
  flex-grow: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0 var(--space-3);
}

.titlebar__target-button,
.titlebar__target-input {
  min-width: 14rem;
  max-width: min(34rem, 100%);
  height: calc(var(--titlebar-height) * 0.68);
  padding: 0 var(--space-2);
  color: var(--color-text-primary);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-control);
  background: var(--color-input);
  font: inherit;
  font-size: var(--font-size-xs);
  line-height: var(--line-height-xs);
}

.titlebar__target-button {
  width: fit-content;
}

.titlebar__target-button:disabled {
  cursor: not-allowed;
  opacity: 0.4;
}

.titlebar__button > * {
  text-align: center;
}
.titlebar__button {
  border-radius: 100%;
  width: calc(var(--titlebar-height) * 0.75);
  height: calc(var(--titlebar-height) * 0.75);
  margin: calc(var(--titlebar-height) * 0.1);
  padding: 0;
  font-size: 1em;
}

.titlebar__icon {
  width: 1em;
  height: 1em;
  fill: none;
  stroke: currentColor;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.35;
}

.titlebar__button--close:hover {
  color: #ffffff;
  background: #c42b1c;
}

:global(html[data-platform="macos"]) .titlebar__leading {
  padding-left: 4.75em;
}

:global(html[data-platform="macos"]) .titlebar__controls {
  visibility: hidden;
}
</style>
