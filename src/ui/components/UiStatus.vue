<script setup lang="ts">
withDefaults(
  defineProps<{
    tone?: "neutral" | "info" | "success" | "warning" | "danger";
  }>(),
  {
    tone: "neutral",
  },
);
</script>

<template>
  <span class="ui-status" :class="`ui-status--${tone}`">
    <span class="ui-status__dot" aria-hidden="true" />
    <slot />
  </span>
</template>

<style scoped>
.ui-status {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  color: var(--status-color, var(--color-text-secondary));
  font-size: var(--font-size-xs);
}

.ui-status__dot {
  width: 0.45rem;
  height: 0.45rem;
  border-radius: 50%;
  background: currentColor;
}

.ui-status--info {
  --status-color: var(--color-info);
}

.ui-status--success {
  --status-color: var(--color-success);
}

.ui-status--warning {
  --status-color: var(--color-warning);
}

.ui-status--danger {
  --status-color: var(--color-danger);
}
</style>
