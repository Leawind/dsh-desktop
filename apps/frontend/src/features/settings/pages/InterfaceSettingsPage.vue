<script setup lang="ts">
import { ElButton, ElInputNumber } from "element-plus";

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

const pageScaleFactor = 1.25;

function clampPageScale(value: number): number {
  return Math.min(200, Math.max(50, value));
}

function normalizePageScale(value: number): number {
  return Number.parseFloat(value.toPrecision(12));
}

function setPageScale(value: number | null | undefined): void {
  if (typeof value === "number" && Number.isFinite(value)) {
    pageScalePercent.value = clampPageScale(normalizePageScale(value));
  }
}

function adjustPageScale(direction: 1 | -1): void {
  setPageScale(pageScalePercent.value * pageScaleFactor ** direction);
}

function formatPageScale(value: string | number): string {
  const numericValue = Number(value);
  return Number.isFinite(numericValue) ? String(Math.round(numericValue)) : "";
}

function parsePageScale(value: string): string {
  return value;
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
      <div class="settings-page__page-scale-control">
        <ElButton
          :aria-label="$t('settings.decreasePageScale')"
          :disabled="pageScalePercent <= 50"
          @click="adjustPageScale(-1)"
        >
          −
        </ElButton>
        <ElInputNumber
          id="page-scale"
          :model-value="pageScalePercent"
          :min="50"
          :max="200"
          :formatter="formatPageScale"
          :parser="parsePageScale"
          :aria-label="$t('settings.pageScale')"
          :controls="false"
          :value-on-clear="50"
          @update:model-value="setPageScale"
        />
        <ElButton
          :aria-label="$t('settings.increasePageScale')"
          :disabled="pageScalePercent >= 200"
          @click="adjustPageScale(1)"
        >
          +
        </ElButton>
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

.settings-page__page-scale-control {
  display: flex;
  width: 160px;
}

.settings-page__page-scale-control :deep(.el-button) {
  width: 40px;
  margin: 0;
  border-radius: 0;
}

.settings-page__page-scale-control :deep(.el-button:first-child) {
  border-radius: var(--radius-control) 0 0 var(--radius-control);
}

.settings-page__page-scale-control :deep(.el-button:last-child) {
  border-radius: 0 var(--radius-control) var(--radius-control) 0;
}

.settings-page__page-scale-control :deep(.el-input-number) {
  width: 80px;
}

.settings-page__page-scale-control :deep(.el-input__wrapper) {
  border-radius: 0;
}
</style>
