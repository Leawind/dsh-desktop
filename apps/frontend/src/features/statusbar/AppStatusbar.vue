<script setup lang="ts">
import { RefreshRight, Setting } from "@element-plus/icons-vue";
import { computed } from "vue";
import { useI18n } from "vue-i18n";

import { UiButton, UiStatus } from "@dsh-desktop/ui";

import type { ServiceStatus } from "@/types/desktop";

const props = defineProps<{
  refreshAction: "refresh" | "retry" | null;
  status: ServiceStatus;
  targetUrl: string;
  message: string | null;
}>();

defineEmits<{
  settings: [];
  refresh: [];
}>();

const { t } = useI18n();

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
const statusTone = computed(() => {
  switch (props.status) {
    case "running":
      return "success";
    case "failed":
    case "unreachable":
      return "danger";
    default:
      return "info";
  }
});
</script>

<template>
  <footer class="statusbar">
    <div class="statusbar__left">
      <UiButton
        variant="ghost"
        size="small"
        class="statusbar__button"
        :aria-label="t('window.settings')"
        @click="$emit('settings')"
      >
        <Setting aria-hidden="true" />
      </UiButton>
      <UiButton
        variant="ghost"
        size="small"
        class="statusbar__button"
        :disabled="refreshAction === null"
        :aria-label="t(refreshAction === 'retry' ? 'common.retry' : 'window.refresh')"
        :title="t(refreshAction === 'retry' ? 'common.retry' : 'window.refresh')"
        @click="$emit('refresh')"
      >
        <RefreshRight aria-hidden="true" />
      </UiButton>
    </div>

    <div class="statusbar__center">
      <p v-if="message" aria-live="polite">{{ message }}</p>
    </div>

    <div class="statusbar__right">
      <span class="statusbar__target" :title="props.targetUrl || undefined">
        {{ targetLabel }}
      </span>
      <UiStatus :tone="statusTone" :animated="status === 'starting'">
        {{ t(`service.status.${status}`) }}
      </UiStatus>
    </div>
  </footer>
</template>

<style scoped>
.statusbar {
  font-size: 1rem;

  width: 100%;
  height: 2em;
  padding: 0;

  display: flex;
  flex-flow: row nowrap;
  justify-content: space-between;
  align-items: stretch;

  color: var(--color-text-secondary);
  background: var(--color-surface);

  border-top: 1px solid var(--color-border);
}

.statusbar__left,
.statusbar__right {
  display: flex;
  flex-flow: row nowrap;
  align-items: center;
  justify-content: start;
}

.statusbar__left > .statusbar__button {
  margin: 0;
  padding: 1.55em;
  border-radius: 0;
}
.statusbar__left > .statusbar__button > :deep(svg) {
  width: 1.4em;
  height: 1.4em;
}

.statusbar__center {
  height: 100%;

  margin: 0;
  padding: 0;

  display: flex;
  flex-flow: row nowrap;
  justify-content: center;
  align-items: center;

  overflow: hidden;

  font-size: 0.9em;
  text-align: center;

  color: var(--color-text-secondary);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.statusbar__right {
  padding: 1em;
}

.statusbar__target {
  max-width: min(24rem, 35vw);
  padding: var(--space-1) var(--space-2);
  overflow: hidden;
  color: var(--color-text-primary);
  border-radius: var(--radius-control);
  font-size: var(--font-size-xs);
  line-height: var(--line-height-xs);
  text-overflow: ellipsis;
  white-space: nowrap;
}

@media (max-width: 42rem) {
  .statusbar {
    gap: var(--space-2);
  }

  .statusbar__center {
    display: none;
  }

  .statusbar__target {
    max-width: 58vw;
  }
}
</style>
