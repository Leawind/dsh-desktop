<script setup lang="ts">
export type SelectOption = {
  value: string;
  label: string;
};

const model = defineModel<string>({ required: true });

defineProps<{
  id?: string;
  options: readonly SelectOption[];
  disabled?: boolean;
}>();
</script>

<template>
  <select :id="id" v-model="model" class="ui-select" :disabled="disabled">
    <option v-for="option in options" :key="option.value" :value="option.value">
      {{ option.label }}
    </option>
  </select>
</template>

<style scoped>
.ui-select {
  width: 100%;
  min-height: 2.25rem;
  padding: 0 var(--space-8) 0 var(--space-3);
  color: var(--color-text-primary);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-control);
  outline: none;
  background: var(--color-input);
  font: inherit;
  font-size: var(--font-size-sm);
}

.ui-select:focus {
  border-color: var(--color-focus);
  box-shadow: 0 0 0 3px var(--color-focus-ring);
}
</style>
