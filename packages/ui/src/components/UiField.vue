<script setup lang="ts">
defineProps<{
  label: string;
  hint?: string;
  error?: string;
  inputId?: string;
}>();
</script>

<template>
  <label class="ui-field" :for="inputId">
    <span class="ui-field__label">{{ label }}</span>
    <slot />
    <span v-if="error" class="ui-field__error">{{ error }}</span>
    <span v-else-if="hint" class="ui-field__hint">{{ hint }}</span>
  </label>
</template>

<style scoped>
.ui-field {
  display: grid;
  gap: var(--space-2);
  color: var(--color-text-primary);
}

.ui-field__label {
  font-size: var(--font-size-sm);
  font-weight: 500;
  line-height: var(--line-height-sm);
}

.ui-field__hint,
.ui-field__error {
  font-size: var(--font-size-xs);
  line-height: var(--line-height-xs);
}

.ui-field__hint {
  color: var(--color-text-secondary);
}

.ui-field__error {
  color: var(--color-danger);
}
</style>
