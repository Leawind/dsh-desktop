<script setup lang="ts">
const matrixCells = [
  [0, 0],
  [4, 0],
  [8, 0],
  [8, 4],
  [8, 8],
  [4, 8],
  [0, 8],
  [0, 4],
] as const;

withDefaults(
  defineProps<{
    tone?: "neutral" | "info" | "success" | "warning" | "danger";
    animated?: boolean;
  }>(),
  {
    tone: "neutral",
    animated: false,
  },
);
</script>

<template>
  <span class="ui-status" :class="`ui-status--${tone}`">
    <svg
      v-if="animated"
      class="ui-status__matrix"
      width="10"
      height="10"
      viewBox="0 0 10 10"
      shape-rendering="crispEdges"
      aria-hidden="true"
    >
      <rect
        v-for="([x, y], index) in matrixCells"
        :key="`${x}-${y}`"
        class="ui-status__cell"
        :x="x"
        :y="y"
        width="2"
        height="2"
        :style="{ animationDelay: `${(index - matrixCells.length) * 125}ms` }"
      />
    </svg>
    <span v-else class="ui-status__dot" aria-hidden="true" />
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
  line-height: var(--line-height-xs);
}

.ui-status__dot {
  position: relative;
  width: 0.625rem;
  height: 0.625rem;
  flex: none;
}

.ui-status__dot::before,
.ui-status__dot::after {
  position: absolute;
  border-radius: 50%;
  background: currentColor;
  content: "";
}

.ui-status__dot::before {
  inset: 0;
  opacity: 0.1;
}

.ui-status__dot::after {
  inset: 20%;
}

.ui-status__matrix {
  flex: none;
}

.ui-status__cell {
  fill: currentColor;
  opacity: 0.15;
  animation: ui-status-chase 1s infinite;
}

@keyframes ui-status-chase {
  0%,
  12.4% {
    opacity: 1;
  }

  12.5%,
  24.9% {
    opacity: 0.6;
  }

  25%,
  37.4% {
    opacity: 0.35;
  }

  37.5%,
  100% {
    opacity: 0.15;
  }
}

@media (prefers-reduced-motion: reduce) {
  .ui-status__cell {
    animation: none;
  }
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
