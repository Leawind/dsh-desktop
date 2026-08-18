<script setup lang="ts">
import { UiSettingRow, UiStatus } from "@dsh-desktop/ui";
import type { AppMetadataSnapshot, DistributionSnapshot } from "@/types/desktop";

const props = defineProps<{
  appMetadata: AppMetadataSnapshot;
  distribution: DistributionSnapshot;
}>();
</script>

<template>
  <section
    id="settings-panel-about"
    class="settings-page"
    role="tabpanel"
    aria-labelledby="settings-tab-about"
  >
    <UiSettingRow :label="appMetadata.name" :hint="appMetadata.identifier">
      <UiStatus tone="info">v{{ appMetadata.version }}</UiStatus>
    </UiSettingRow>
    <UiSettingRow
      :label="$t('runtime.variant')"
      :hint="$t(`runtime.variantHint.${distribution.variant}`)"
    >
      <UiStatus tone="info">{{ $t(`runtime.variantName.${distribution.variant}`) }}</UiStatus>
    </UiSettingRow>
    <template v-if="distribution.builtInRuntime">
      <UiSettingRow
        :label="$t('about.builtInRuntime')"
        :hint="
          $t('runtime.builtInDetails', {
            runtimeId: distribution.builtInRuntime.runtimeId,
            node: distribution.builtInRuntime.nodeVersion,
            pnpm: distribution.builtInRuntime.pnpmVersion,
          })
        "
      >
        <UiStatus :tone="distribution.builtInRuntime.installed ? 'success' : 'info'">
          DSH {{ distribution.builtInRuntime.dshVersion }} ·
          {{
            $t(
              distribution.builtInRuntime.installed
                ? "runtime.installed"
                : "runtime.readyToInstall",
            )
          }}
        </UiStatus>
      </UiSettingRow>
    </template>
    <UiSettingRow v-else :label="$t('about.builtInRuntime')" :hint="$t('runtime.notIncluded')">
      <UiStatus tone="info">{{ $t("runtime.notIncludedStatus") }}</UiStatus>
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

.settings-page__inline-control {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: var(--space-2);
}

.settings-page__message,
.settings-page__error,
.settings-page__details {
  margin: 0;
  padding: var(--space-3) 0;
  font-size: var(--font-size-sm);
}

.settings-page__message,
.settings-page__details {
  color: var(--color-text-secondary);
}

.settings-page__error {
  color: var(--color-danger);
}

.settings-page__details pre {
  max-height: 10rem;
  overflow: auto;
  white-space: pre-wrap;
}
</style>
