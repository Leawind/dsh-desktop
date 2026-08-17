<script setup lang="ts">
import { UiInput, UiSelect, UiSettingRow } from "@dsh-desktop/ui";

import type { DshHome, DshSource } from "@/types/desktop";

const props = defineProps<{
  sourceOptions: { value: string; label: string; disabled?: boolean }[];
  homeOptions: { value: string; label: string }[];
}>();

const emit = defineEmits<{
  select: [];
}>();

const sourceType = defineModel<DshSource["type"]>("sourceType", { required: true });
const customExecutable = defineModel<string>("customExecutable", { required: true });
const npxVersion = defineModel<string>("npxVersion", { required: true });
const homeType = defineModel<DshHome["type"]>("homeType", { required: true });
const customDshHome = defineModel<string>("customDshHome", { required: true });
</script>

<template>
  <section
    id="settings-panel-dsh"
    class="settings-page"
    role="tabpanel"
    aria-labelledby="settings-tab-dsh"
  >
    <UiSettingRow :label="$t('settings.source.label')" :hint="$t('settings.source.hint')">
      <UiSelect
        v-model="sourceType"
        variant="pill"
        :options="props.sourceOptions"
        @change="emit('select')"
      />
    </UiSettingRow>
    <UiSettingRow
      v-if="sourceType === 'custom'"
      control-id="dsh-executable"
      :label="$t('settings.executable')"
      :hint="$t('settings.executableHint')"
    >
      <div class="settings-page__wide-control">
        <UiInput id="dsh-executable" v-model="customExecutable" />
      </div>
    </UiSettingRow>
    <UiSettingRow
      v-if="sourceType === 'npx'"
      control-id="npx-dsh-version"
      :label="$t('settings.npxVersion')"
      :hint="$t('settings.npxVersionHint')"
    >
      <div class="settings-page__wide-control">
        <UiInput id="npx-dsh-version" v-model="npxVersion" />
      </div>
    </UiSettingRow>
    <UiSettingRow :label="$t('settings.home.label')" :hint="$t('settings.home.hint')">
      <UiSelect
        v-model="homeType"
        variant="pill"
        :options="props.homeOptions"
        @change="emit('select')"
      />
    </UiSettingRow>
    <UiSettingRow
      v-if="homeType === 'custom'"
      control-id="dsh-home"
      :label="$t('settings.home.path')"
      :hint="$t('settings.home.pathHint')"
    >
      <div class="settings-page__wide-control">
        <UiInput id="dsh-home" v-model="customDshHome" />
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

.settings-page__wide-control {
  width: 17.5rem;
}

@media (max-width: 40rem) {
  .settings-page__wide-control {
    width: 100%;
  }
}
</style>
