<script setup lang="ts">
import { UiInput, UiSettingRow } from "@dsh-desktop/ui";

defineProps<{
  error: string;
}>();

const url = defineModel<string>("url", { required: true });
</script>

<template>
  <section
    id="settings-panel-window"
    class="settings-page"
    role="tabpanel"
    aria-labelledby="settings-tab-window"
  >
    <UiSettingRow control-id="current-url" :label="$t('window.url')" :hint="$t('window.urlHint')">
      <div class="settings-page__control-stack">
        <UiInput id="current-url" v-model="url" type="url" />
        <span v-if="error" class="settings-page__error">{{ error }}</span>
      </div>
    </UiSettingRow>
  </section>
</template>

<style scoped>
.settings-page {
  width: 100%;
}

.settings-page :deep(.ui-setting-row:last-child) {
  border-bottom: 0;
}

.settings-page__control-stack {
  display: flex;
  width: 280px;
  flex-direction: column;
  gap: var(--space-1);
}

.settings-page__error {
  color: var(--color-danger);
  font-size: var(--font-size-xs);
  line-height: var(--line-height-xs);
}

@media (max-width: 40rem) {
  .settings-page__control-stack {
    width: 100%;
  }
}
</style>
