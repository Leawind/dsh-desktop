<script setup lang="ts">
import { Close, FullScreen, Minus, RefreshRight, Setting } from "@element-plus/icons-vue";
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
      >
        <RefreshRight class="titlebar__icon" aria-hidden="true" />
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
        <Minus class="titlebar__icon" aria-hidden="true" />
      </UiButton>
      <UiButton
        variant="ghost"
        size="small"
        class="titlebar__button"
        :aria-label="t('window.maximize')"
        @click.stop="desktopBridge.window.toggleMaximize()"
      >
        <FullScreen class="titlebar__icon" aria-hidden="true" />
      </UiButton>
      <UiButton
        variant="ghost"
        size="small"
        class="titlebar__button titlebar__button--close"
        :aria-label="t('window.close')"
        @click.stop="desktopBridge.window.close()"
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
