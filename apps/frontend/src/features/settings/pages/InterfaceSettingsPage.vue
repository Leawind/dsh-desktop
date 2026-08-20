<script setup lang="ts">
import { ElSelect } from "element-plus";

import { UiSettingRow } from "@dsh-desktop/ui";

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
      <!-- @vue-ignore Element Plus 2.14.4 options typing is incompatible with TypeScript 6. -->
      <ElSelect
        v-model="locale"
        class="dsh-select dsh-select--pill"
        popper-class="dsh-select-popper"
        :options="localeOptions"
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
</style>
