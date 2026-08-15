<script setup lang="ts">
const model = defineModel<string>({ required: true });

withDefaults(
  defineProps<{
    id?: string;
    type?: "text" | "url" | "number";
    placeholder?: string;
    disabled?: boolean;
    autocomplete?: string;
  }>(),
  {
    type: "text",
    disabled: false,
    autocomplete: "off",
  },
);
</script>

<template>
  <input
    :id="id"
    v-model="model"
    class="ui-input"
    :type="type"
    :placeholder="placeholder"
    :disabled="disabled"
    :autocomplete="autocomplete"
  />
</template>

<style scoped>
.ui-input {
  width: 100%;
  min-height: 2.25rem;
  padding: 0 var(--space-3);
  color: var(--color-text-primary);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-control);
  outline: none;
  background: var(--color-input);
  font: inherit;
  font-size: var(--font-size-sm);
  transition:
    border-color var(--duration-fast),
    box-shadow var(--duration-fast);
}

.ui-input::placeholder {
  color: var(--color-text-placeholder);
}

.ui-input:focus {
  border-color: var(--color-focus);
  box-shadow: 0 0 0 3px var(--color-focus-ring);
}
</style>
