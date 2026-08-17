<script setup lang="ts">
import { ElSlider } from "element-plus";

import { UiSelect, UiSettingRow } from "@dsh-desktop/ui";

import type { AppLocale } from "@/types/desktop";

const emit = defineEmits<{
  select: [];
}>();

const locale = defineModel<AppLocale>("locale", { required: true });
const pageScalePercent = defineModel<number>("pageScalePercent", { required: true });

defineProps<{
  localeOptions: { value: string; label: string }[];
}>();

function formatPageScaleTooltip(value: number): string {
  return `${value}%`;
}
</script>

<template>
  <section
    id="settings-panel-interface"
    class="settings-page"
    role="tabpanel"
    aria-labelledby="settings-tab-interface"
  >
    <UiSettingRow class="settings-page__language-row" :label="$t('settings.language')">
      <UiSelect v-model="locale" variant="pill" :options="localeOptions" @change="emit('select')" />
    </UiSettingRow>
    <UiSettingRow
      class="settings-page__page-scale-row"
      control-id="page-scale"
      :label="$t('settings.pageScale')"
      :hint="$t('settings.pageScaleHint')"
    >
      <ElSlider
        id="page-scale"
        class="settings-page__page-scale-control"
        v-model="pageScalePercent"
        :min="50"
        :max="400"
        :step="25"
        show-stops
        :format-tooltip="formatPageScaleTooltip"
        :aria-label="$t('settings.pageScale')"
        @change="emit('select')"
      />
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

.settings-page__page-scale-row {
  align-items: stretch;
  flex-direction: column;
  gap: var(--space-3);
}

.settings-page__page-scale-row :deep(.ui-setting-row__text) {
  padding-right: 0;
}

.settings-page__page-scale-control {
  width: 100%;
}

.settings-page__language-row :deep(.ui-setting-row__control) {
  align-self: flex-end;
}
</style>
