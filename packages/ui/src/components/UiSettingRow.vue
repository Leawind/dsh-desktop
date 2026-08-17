<script setup lang="ts">
defineProps<{
  label: string;
  hint?: string;
  controlId?: string;
}>();
</script>

<template>
  <div class="ui-setting-row">
    <div class="ui-setting-row__text">
      <label v-if="controlId" class="ui-setting-row__label" :for="controlId">{{ label }}</label>
      <div v-else class="ui-setting-row__label">{{ label }}</div>
      <div v-if="hint" class="ui-setting-row__hint">{{ hint }}</div>
    </div>
    <div class="ui-setting-row__control">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.ui-setting-row {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-4) 0;
  border-bottom: 0.0625rem solid var(--color-border);
}

.ui-setting-row__text {
  display: flex;
  min-width: 0;
  flex: 1;
  flex-direction: column;
  gap: var(--space-1);
  padding-right: 3rem;
}

.ui-setting-row__label {
  color: var(--color-text-primary);
  font-size: var(--font-size-sm);
  font-weight: 400;
  line-height: var(--line-height-sm);
  overflow-wrap: anywhere;
}

.ui-setting-row__hint {
  color: var(--color-text-secondary);
  font-size: var(--font-size-xs);
  line-height: var(--line-height-xs);
}

.ui-setting-row__control {
  min-width: 0;
  flex: none;
}

@media (max-width: 30rem) {
  .ui-setting-row {
    align-items: stretch;
    flex-direction: column;
    gap: var(--space-3);
  }

  .ui-setting-row__text {
    padding-right: 0;
  }

  .ui-setting-row__control {
    width: 100%;
  }
}
</style>
