<script setup lang="ts">
import { isTauri } from "@tauri-apps/api/core";
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";
import { computed, ref } from "vue";

import { UiButton, UiSettingRow, UiStatus } from "@dsh-desktop/ui";

import { desktopBridge } from "@/bridge/desktop";
import type { AppMetadataSnapshot, DistributionSnapshot } from "@/types/desktop";

const props = defineProps<{
  appMetadata: AppMetadataSnapshot;
  distribution: DistributionSnapshot;
}>();

const candidate = ref<Update | null>(null);
const checked = ref(false);
const checking = ref(false);
const installing = ref(false);
const downloadedBytes = ref(0);
const contentLength = ref<number | null>(null);
const updateError = ref<string | null>(null);
const canCheckForUpdate = computed(() => isTauri());

function updateErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

async function checkForUpdate(): Promise<void> {
  if (!canCheckForUpdate.value) return;
  checking.value = true;
  checked.value = false;
  candidate.value = null;
  updateError.value = null;
  try {
    candidate.value = await check({ timeout: 30_000 });
    checked.value = true;
  } catch (error) {
    updateError.value = updateErrorMessage(error);
  } finally {
    checking.value = false;
  }
}

async function installUpdate(): Promise<void> {
  const update = candidate.value;
  if (!update) return;
  installing.value = true;
  downloadedBytes.value = 0;
  contentLength.value = null;
  updateError.value = null;
  try {
    await update.downloadAndInstall((event: DownloadEvent) => {
      if (event.event === "Started") contentLength.value = event.data.contentLength ?? null;
      else if (event.event === "Progress") downloadedBytes.value += event.data.chunkLength;
    });
    await desktopBridge.restartApp();
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
        <UiButton
          size="small"
          :disabled="!canCheckForUpdate || checking || installing"
          @click="checkForUpdate"
        >
          {{ $t(checking ? "about.checkingForUpdate" : "about.checkForUpdate") }}
        </UiButton>
      </div>
    </UiSettingRow>
    <UiSettingRow
      v-if="candidate"
      :label="$t('about.updateAvailable', { version: candidate.version })"
      :hint="candidate.body || $t('about.updateNotesUnavailable')"
    >
      <div class="settings-page__inline-control">
        <UiStatus tone="info">v{{ candidate.version }}</UiStatus>
        <UiButton variant="primary" size="small" :disabled="installing" @click="installUpdate">
          {{ $t(installing ? "about.installingUpdate" : "about.installAndRestart") }}
        </UiButton>
      </div>
    </UiSettingRow>
    <p v-else-if="checked" class="settings-page__message">{{ $t("about.upToDate") }}</p>
    <p v-if="installing" class="settings-page__message">
      {{
        contentLength === null
          ? $t("about.downloadingUpdate")
          : $t("about.downloadProgress", { downloaded: downloadedBytes, total: contentLength })
      }}
    </p>
    <p v-if="updateError" class="settings-page__error">{{ $t("about.updateFailed") }}</p>
    <details v-if="updateError" class="settings-page__details">
      <summary>{{ $t("common.details") }}</summary>
      <pre>{{ updateError }}</pre>
    </details>
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
