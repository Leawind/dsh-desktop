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
  height: var(--control-height);
  padding: 0 var(--space-2);
  color: var(--color-text-primary);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-control);
  outline: none;
  background: var(--color-input);
  font: inherit;
  font-size: var(--font-size-sm);
  line-height: var(--line-height-sm);
}

.ui-input::placeholder {
  color: var(--color-text-placeholder);
}

.ui-input:focus {
  border-color: var(--color-focus);
}

.ui-input:disabled {
  cursor: not-allowed;
  opacity: 0.4;
}
</style>
