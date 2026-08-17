<script setup lang="ts">
import { ElInputNumber } from "element-plus";

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

function clampPageScale(value: number): number {
  return Math.min(200, Math.max(50, value));
}

function setPageScale(value: number | null | undefined): void {
  if (typeof value === "number" && Number.isFinite(value)) {
    pageScalePercent.value = clampPageScale(Math.round(value));
  }
}
</script>

<template>
  <section
    id="settings-panel-interface"
    class="settings-page"
    role="tabpanel"
    aria-labelledby="settings-tab-interface"
  >
    <UiSettingRow :label="$t('settings.language')">
      <UiSelect v-model="locale" variant="pill" :options="localeOptions" @change="emit('select')" />
    </UiSettingRow>
    <UiSettingRow
      control-id="page-scale"
      :label="$t('settings.pageScale')"
      :hint="$t('settings.pageScaleHint')"
    >
      <ElInputNumber
        id="page-scale"
        class="settings-page__page-scale-control"
        :model-value="pageScalePercent"
        :min="50"
        :max="200"
        :step="25"
        :precision="0"
        :aria-label="$t('settings.pageScale')"
        :value-on-clear="50"
        @update:model-value="setPageScale"
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

.settings-page__page-scale-control {
  width: 160px;
}
</style>
