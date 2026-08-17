<script setup lang="ts">
const emit = defineEmits<{
  click: [event: MouseEvent];
}>();

withDefaults(
  defineProps<{
    variant?: "primary" | "secondary" | "ghost" | "danger";
    size?: "small" | "medium";
    disabled?: boolean;
    type?: "button" | "submit" | "reset";
  }>(),
  {
    variant: "secondary",
    size: "medium",
    disabled: false,
    type: "button",
  },
);
</script>

<template>
  <button
    class="ui-button"
    :class="[`ui-button--${variant}`, `ui-button--${size}`]"
    :disabled="disabled"
    :type="type"
    @click="emit('click', $event)"
  >
    <slot />
  </button>
</template>

<style scoped>
.ui-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 0;
  border: 0.0625rem solid transparent;
  gap: var(--space-1);
  border-radius: var(--radius-button-medium);
  font: inherit;
  font-size: var(--font-size-sm);
  font-weight: 500;
  line-height: var(--line-height-sm);
  cursor: pointer;
}

.ui-button--medium {
  height: var(--button-height-medium);
  padding: 0 0.875rem;
}

.ui-button--small {
  height: var(--button-height-small);
  padding: 0 0.625rem;
  border-radius: var(--radius-button-small);
  font-size: var(--font-size-xs);
  line-height: var(--line-height-xs);
}

.ui-button--primary {
  color: var(--color-accent-contrast);
  background: var(--color-accent);
}

.ui-button--primary:hover:not(:disabled) {
  background: var(--color-accent-hover);
}

.ui-button--secondary {
  color: var(--color-text-primary);
  border-color: var(--color-border);
  background: transparent;
}

.ui-button--secondary:hover:not(:disabled),
.ui-button--ghost:hover:not(:disabled) {
  background: var(--color-interactive-hover);
}

.ui-button--secondary:active:not(:disabled),
.ui-button--ghost:active:not(:disabled) {
  background: var(--color-interactive-active);
}

.ui-button--ghost {
  color: var(--color-text-secondary);
  background: transparent;
}

.ui-button--danger {
  color: var(--color-danger-contrast);
  background: var(--color-danger);
}

.ui-button:disabled {
  cursor: not-allowed;
  opacity: 0.4;
}
</style>
