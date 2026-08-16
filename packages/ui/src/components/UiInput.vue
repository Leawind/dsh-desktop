<script setup lang="ts">
import { ref } from "vue";

const model = defineModel<string>({ required: true });
const input = ref<HTMLInputElement | null>(null);

withDefaults(
  defineProps<{
    id?: string;
    type?: "text" | "url" | "number";
    placeholder?: string;
    disabled?: boolean;
    autocomplete?: string;
    contentSized?: boolean;
  }>(),
  {
    type: "text",
    disabled: false,
    autocomplete: "off",
    contentSized: false,
  },
);

defineExpose({
  focus: () => input.value?.focus(),
});
</script>

<template>
  <input
    ref="input"
    :id="id"
    v-model="model"
    class="ui-input"
    :class="{ 'ui-input--content-sized': contentSized }"
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

.ui-input--content-sized {
  width: auto;
  field-sizing: content;
}
</style>
