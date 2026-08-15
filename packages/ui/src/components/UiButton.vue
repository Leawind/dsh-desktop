<script setup lang="ts">
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
  border: 1px solid transparent;
  border-radius: var(--radius-control);
  font: inherit;
  font-size: var(--font-size-sm);
  font-weight: 550;
  line-height: 1;
  cursor: pointer;
  transition:
    background-color var(--duration-fast),
    border-color var(--duration-fast),
    color var(--duration-fast);
}

.ui-button--medium {
  min-height: 2.25rem;
  padding: 0 var(--space-4);
}

.ui-button--small {
  min-height: 1.875rem;
  padding: 0 var(--space-3);
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
  background: var(--color-control);
}

.ui-button--secondary:hover:not(:disabled),
.ui-button--ghost:hover:not(:disabled) {
  background: var(--color-control-hover);
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
  cursor: default;
  opacity: 0.5;
}
</style>
