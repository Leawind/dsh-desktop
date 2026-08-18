<script setup lang="ts">
import { UiSelect, UiSettingRow } from "@dsh-desktop/ui";

import type { AppLocale } from "@/types/desktop";

const emit = defineEmits<{
  select: [];
}>();

const locale = defineModel<AppLocale>("locale", { required: true });

defineProps<{
  localeOptions: { value: string; label: string }[];
}>();
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
  </section>
</template>

<style scoped>
.settings-page {
  width: 100%;
}

.settings-page :deep(.ui-setting-row:last-child) {
  border-bottom: 0;
}
</style>
