<script setup lang="ts">
import { useI18n } from "vue-i18n";

import { UiButton } from "@dsh-desktop/ui";

import { desktopBridge } from "@/bridge/desktop";

defineEmits<{
  settings: [];
}>();

const { t } = useI18n();
</script>

<template>
  <header class="titlebar" @dblclick="desktopBridge.window.toggleMaximize()">
    <div class="titlebar__leading">
      <UiButton
        variant="ghost"
        size="small"
        class="titlebar__button"
        :aria-label="t('window.settings')"
        @click.stop="$emit('settings')"
      >
        <span aria-hidden="true">⚙</span>
      </UiButton>
    </div>
    <button
      class="titlebar__drag"
      type="button"
      :aria-label="t('app.name')"
      @mousedown.left="desktopBridge.window.startDragging()"
    >
      {{ t("app.name") }}
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
  display: grid;
  height: var(--titlebar-height);
  grid-template-columns: 8rem minmax(0, 1fr) 8rem;
  align-items: stretch;
  color: var(--color-text-secondary);
  border-bottom: 1px solid var(--color-border);
  background: var(--color-surface);
  user-select: none;
}

.titlebar__leading,
.titlebar__controls {
  display: flex;
  align-items: stretch;
}

.titlebar__controls {
  justify-content: flex-end;
}

.titlebar__drag {
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
  width: 2.75rem;
  height: 100%;
  padding: 0;
  border-radius: 0;
  font-size: 1rem;
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
