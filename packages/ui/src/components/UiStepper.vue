<script setup lang="ts">
import { computed } from "vue";

const model = defineModel<number>({ required: true });

type StepperAdjustment = { type: "offset"; amount: number } | { type: "factor"; factor: number };

const props = withDefaults(
  defineProps<{
    id?: string;
    min: number;
    max: number;
    adjustment?: StepperAdjustment;
    valueLabel: string;
    decreaseLabel: string;
    increaseLabel: string;
  }>(),
  {
    adjustment: () => ({ type: "offset", amount: 1 }),
  },
);

const adjustmentIsValid = computed(() => {
  if (props.adjustment.type === "offset") {
    return Number.isFinite(props.adjustment.amount) && props.adjustment.amount > 0;
  }
  return Number.isFinite(props.adjustment.factor) && props.adjustment.factor > 1;
});

const canDecrease = computed(() => adjustmentIsValid.value && model.value > props.min);
const canIncrease = computed(() => adjustmentIsValid.value && model.value < props.max);
const displayValue = computed(() => Math.round(model.value));

function clamp(value: number): number {
  return Math.min(props.max, Math.max(props.min, value));
}

function normalize(value: number): number {
  return Number.parseFloat(value.toPrecision(12));
}

function adjustedValue(direction: 1 | -1): number {
  const adjustment = props.adjustment;
  const value =
    adjustment.type === "offset"
      ? model.value + direction * adjustment.amount
      : model.value * adjustment.factor ** direction;
  return clamp(normalize(value));
}

function decrease(): void {
  if (canDecrease.value) model.value = adjustedValue(-1);
}

function increase(): void {
  if (canIncrease.value) model.value = adjustedValue(1);
}

function updateFromInput(event: Event): void {
  const input = event.target as HTMLInputElement;
  const value = input.valueAsNumber;
  if (!Number.isFinite(value)) {
    input.value = String(displayValue.value);
    return;
  }
  model.value = clamp(normalize(value));
}
</script>

<template>
  <div class="ui-stepper" role="group">
    <button
      class="ui-stepper__button"
      type="button"
      :aria-label="decreaseLabel"
      :disabled="!canDecrease"
      @click="decrease"
    >
      <span aria-hidden="true">−</span>
    </button>
    <input
      :id="id"
      class="ui-stepper__input"
      type="number"
      :value="displayValue"
      :min="min"
      :max="max"
      step="1"
      :aria-label="valueLabel"
      @change="updateFromInput"
    />
    <button
      class="ui-stepper__button"
      type="button"
      :aria-label="increaseLabel"
      :disabled="!canIncrease"
      @click="increase"
    >
      <span aria-hidden="true">+</span>
    </button>
  </div>
</template>

<style scoped>
.ui-stepper {
  display: grid;
  width: 160px;
  height: var(--control-height);
  grid-template-columns: 40px minmax(0, 1fr) 40px;
  overflow: hidden;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-control);
  background: var(--color-input);
}

.ui-stepper__button {
  display: grid;
  padding: 0;
  place-items: center;
  color: var(--color-text-secondary);
  border: 0;
  background: transparent;
  font: inherit;
  font-size: 1.25rem;
  line-height: 1;
  cursor: pointer;
}

.ui-stepper__button:first-child {
  border-right: 1px solid var(--color-border);
}

.ui-stepper__button:last-child {
  border-left: 1px solid var(--color-border);
}

.ui-stepper__button:hover:not(:disabled) {
  color: var(--color-text-primary);
  background: var(--color-interactive-hover);
}

.ui-stepper__button:active:not(:disabled) {
  background: var(--color-interactive-active);
}

.ui-stepper__button:disabled {
  cursor: not-allowed;
  opacity: 0.4;
}

.ui-stepper__input {
  min-width: 0;
  padding: 0 var(--space-2);
  color: var(--color-text-primary);
  border: 0;
  outline: none;
  background: transparent;
  font: inherit;
  font-size: var(--font-size-sm);
  line-height: var(--line-height-sm);
  text-align: center;
}

.ui-stepper__input:focus {
  box-shadow: inset 0 0 0 1px var(--color-focus);
}
</style>
