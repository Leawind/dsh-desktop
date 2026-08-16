<script setup lang="ts">
import { useI18n } from "vue-i18n";

import { UiButton } from "@dsh-desktop/ui";

import { desktopBridge } from "@/bridge/desktop";

defineEmits<{
  settings: [];
  refresh: [];
}>();

defineProps<{
  refreshAction: "refresh" | "retry" | null;
  title: string;
}>();

const { t } = useI18n();
</script>

<template>
  <header class="titlebar" @dblclick="desktopBridge.window.toggleMaximize()">
    <div class="titlebar__leading">
      <img class="titlebar__app-icon" src="/app-icon.png" alt="" aria-hidden="true" />
      <UiButton
        variant="ghost"
        size="small"
        class="titlebar__button"
        :aria-label="t('window.settings')"
        @click.stop="$emit('settings')"
      >
        <span aria-hidden="true">⚙</span>
      </UiButton>
      <UiButton
        variant="ghost"
        size="small"
        class="titlebar__button"
        :disabled="refreshAction === null"
        :aria-label="t(refreshAction === 'retry' ? 'common.retry' : 'window.refresh')"
        :title="t(refreshAction === 'retry' ? 'common.retry' : 'window.refresh')"
        @click.stop="$emit('refresh')"
      >
        <svg class="titlebar__icon" viewBox="0 0 16 16" aria-hidden="true">
          <path d="M13.25 5.75V2.5M13.25 2.5H10M13.25 2.5A6 6 0 1 0 14 9" />
        </svg>
      </UiButton>
    </div>
    <button
      class="titlebar__drag"
      type="button"
      :aria-label="title"
      @mousedown.left="desktopBridge.window.startDragging()"
    >
      {{ title }}
    </button>
    <div class="titlebar__controls">
      <UiButton
        variant="ghost"
        size="small"
        class="titlebar__button"
        :aria-label="t('window.minimize')"
        @click.stop="desktopBridge.window.minimize()"
      >
        <span aria-hidden="true">−</span>
      </UiButton>
      <UiButton
        variant="ghost"
        size="small"
        class="titlebar__button"
        :aria-label="t('window.maximize')"
        @click.stop="desktopBridge.window.toggleMaximize()"
      >
        <span aria-hidden="true">□</span>
      </UiButton>
      <UiButton
        variant="ghost"
        size="small"
        class="titlebar__button titlebar__button--close"
        :aria-label="t('window.close')"
        @click.stop="desktopBridge.window.close()"
      >
        <span aria-hidden="true">×</span>
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

.titlebar__drag {
  flex-grow: 1;
  overflow: hidden;
  color: var(--color-text-secondary);
  border: 0;
  background: transparent;
  font-size: var(--font-size-xs);
  font-weight: 550;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.titlebar__button {
  width: var(--titlebar-height);
  height: var(--titlebar-height);
  padding: 0;
  margin: 0;
  border-radius: 0;
  font-size: 1rem;
}

.titlebar__icon {
  width: 1rem;
  height: 1rem;
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
  padding-left: 4.75rem;
}

:global(html[data-platform="macos"]) .titlebar__controls {
  visibility: hidden;
}
</style>
