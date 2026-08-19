<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";

import { UiButton, UiSettingRow, UiStatus } from "@dsh-desktop/ui";

import { desktopBridge } from "@/bridge/desktop";
import type { AppMetadataSnapshot, AppUpdateSnapshot, DistributionSnapshot } from "@/types/desktop";

const props = defineProps<{
  appMetadata: AppMetadataSnapshot;
  distribution: DistributionSnapshot;
}>();

const { t } = useI18n();
const appUpdate = ref<AppUpdateSnapshot | null>(null);
const updateError = ref<string | null>(null);
const checking = ref(false);
const installing = ref(false);

function updateErrorMessage(error: unknown): string {
  return t(
    typeof error === "object" && error !== null && "code" in error
      ? String(error.code)
      : "app.error.unknown",
  );
}

async function checkForUpdate(): Promise<void> {
  checking.value = true;
  appUpdate.value = null;
  updateError.value = null;
  try {
    appUpdate.value = await desktopBridge.checkAppUpdate();
  } catch (error) {
    updateError.value = updateErrorMessage(error);
  } finally {
    checking.value = false;
  }
}

async function installUpdate(): Promise<void> {
  installing.value = true;
  updateError.value = null;
  try {
    await desktopBridge.installAppUpdate();
  } catch (error) {
    updateError.value = updateErrorMessage(error);
    installing.value = false;
  }
}
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
    <UiSettingRow :label="$t('about.appUpdate')" :hint="$t('about.appUpdateHint')">
      <div class="settings-page__inline-control">
        <UiButton size="small" :disabled="checking || installing" @click="checkForUpdate">
          {{ $t(checking ? "about.checkingForUpdate" : "about.checkForUpdate") }}
        </UiButton>
      </div>
    </UiSettingRow>
    <UiSettingRow
      v-if="appUpdate?.candidate"
      :label="$t('about.updateAvailable', { version: appUpdate.candidate.version })"
      :hint="appUpdate.candidate.notes || $t('about.updateNotesUnavailable')"
    >
      <div class="settings-page__inline-control">
        <UiStatus tone="info">v{{ appUpdate.candidate.version }}</UiStatus>
        <UiButton variant="primary" size="small" :disabled="installing" @click="installUpdate">
          {{ $t(installing ? "about.downloadingUpdate" : "about.installAndRestart") }}
        </UiButton>
      </div>
    </UiSettingRow>
    <p v-else-if="appUpdate" class="settings-page__message">{{ $t("about.upToDate") }}</p>
    <p v-if="updateError" class="settings-page__error">{{ updateError }}</p>
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
