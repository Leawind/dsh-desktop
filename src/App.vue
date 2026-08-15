<script setup lang="ts">
import { useI18n } from "vue-i18n";

import { UiButton } from "@/ui";

const { t } = useI18n();
</script>

<template>
  <main class="bootstrap-view">
    <section class="bootstrap-card">
      <span class="bootstrap-mark" aria-hidden="true">DSH</span>
      <h1>{{ t("app.name") }}</h1>
      <p>{{ t("app.initializing") }}</p>
      <UiButton>{{ t("common.settings") }}</UiButton>
    </section>
  </main>
</template>

<style scoped>
.bootstrap-view {
  display: grid;
  min-height: 100vh;
  place-items: center;
  padding: var(--space-6);
  background: var(--color-background);
}

.bootstrap-card {
  display: grid;
  width: min(100%, 28rem);
  justify-items: start;
  gap: var(--space-4);
  padding: var(--space-8);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-panel);
  background: var(--color-surface);
  box-shadow: var(--shadow-panel);
}

.bootstrap-card h1,
.bootstrap-card p {
  margin: 0;
}

.bootstrap-card p {
  color: var(--color-text-secondary);
}

.bootstrap-mark {
  color: var(--color-accent);
  font-size: var(--font-size-xs);
  font-weight: 700;
  letter-spacing: 0.12em;
}
</style>
