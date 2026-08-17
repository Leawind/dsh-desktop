<script setup lang="ts">
import { computed, onBeforeUnmount, ref, useId, watch } from "vue";

export type SelectOption = {
  value: string;
  label: string;
  disabled?: boolean;
};

const model = defineModel<string>({ required: true });
const emit = defineEmits<{
  change: [value: string];
}>();
const props = withDefaults(
  defineProps<{
    id?: string;
    options: readonly SelectOption[];
    disabled?: boolean;
    variant?: "field" | "pill";
  }>(),
  {
    disabled: false,
    variant: "field",
  },
);

const generatedId = useId();
const root = ref<HTMLElement | null>(null);
const trigger = ref<HTMLButtonElement | null>(null);
const open = ref(false);
const activeIndex = ref(-1);
const listId = computed(() => `${props.id ?? generatedId}-listbox`);
const selectedOption = computed(() => props.options.find((option) => option.value === model.value));
const activeOptionId = computed(() => {
  if (!open.value || activeIndex.value < 0) return undefined;
  return `${listId.value}-option-${activeIndex.value}`;
});

function isSelectable(index: number): boolean {
  return props.options[index]?.disabled !== true;
}

function firstSelectableIndex(): number {
  return props.options.findIndex((option) => option.disabled !== true);
}

function lastSelectableIndex(): number {
  for (let index = props.options.length - 1; index >= 0; index -= 1) {
    if (isSelectable(index)) return index;
  }
  return -1;
}

function selectedIndex(): number {
  return props.options.findIndex((option) => option.value === model.value && !option.disabled);
}

function openMenu(preferredIndex = selectedIndex()): void {
  if (props.disabled || props.options.length === 0) return;
  activeIndex.value = preferredIndex >= 0 ? preferredIndex : firstSelectableIndex();
  open.value = true;
}

function closeMenu(restoreFocus = false): void {
  open.value = false;
  if (restoreFocus) trigger.value?.focus();
}

function moveActive(direction: 1 | -1): void {
  if (!open.value) {
    const preferredIndex = selectedIndex();
    openMenu(
      preferredIndex >= 0
        ? preferredIndex
        : direction === 1
          ? firstSelectableIndex()
          : lastSelectableIndex(),
    );
    return;
  }
  if (props.options.length === 0) return;

  let index = activeIndex.value;
  for (let attempts = 0; attempts < props.options.length; attempts += 1) {
    index = (index + direction + props.options.length) % props.options.length;
    if (isSelectable(index)) {
      activeIndex.value = index;
      return;
    }
  }
}

function selectOption(option: SelectOption): void {
  if (option.disabled) return;
  model.value = option.value;
  emit("change", option.value);
  closeMenu(true);
}

function selectActive(): void {
  const option = props.options[activeIndex.value];
  if (option) selectOption(option);
}

function onKeydown(event: KeyboardEvent): void {
  switch (event.key) {
    case "ArrowDown":
      event.preventDefault();
      moveActive(1);
      break;
    case "ArrowUp":
      event.preventDefault();
      moveActive(-1);
      break;
    case "Home":
      if (!open.value) return;
      event.preventDefault();
      activeIndex.value = firstSelectableIndex();
      break;
    case "End":
      if (!open.value) return;
      event.preventDefault();
      activeIndex.value = lastSelectableIndex();
      break;
    case "Enter":
    case " ":
      event.preventDefault();
      if (open.value) selectActive();
      else openMenu();
      break;
    case "Escape":
      if (!open.value) return;
      event.preventDefault();
      event.stopPropagation();
      closeMenu();
      break;
    case "Tab":
      closeMenu();
      break;
  }
}

function onDocumentPointerDown(event: PointerEvent): void {
  if (event.target instanceof Node && !root.value?.contains(event.target)) closeMenu();
}

watch(open, (isOpen) => {
  if (isOpen) document.addEventListener("pointerdown", onDocumentPointerDown);
  else document.removeEventListener("pointerdown", onDocumentPointerDown);
});

watch(
  () => props.disabled,
  (disabled) => {
    if (disabled) closeMenu();
  },
);

onBeforeUnmount(() => document.removeEventListener("pointerdown", onDocumentPointerDown));
</script>

<template>
  <span ref="root" class="ui-select" :class="`ui-select--${variant}`">
    <button
      :id="id"
      ref="trigger"
      class="ui-select__trigger"
      type="button"
      role="combobox"
      aria-haspopup="listbox"
      :aria-controls="listId"
      :aria-expanded="open"
      :aria-activedescendant="activeOptionId"
      :disabled="disabled"
      @click="open ? closeMenu() : openMenu()"
      @keydown="onKeydown"
    >
      <span class="ui-select__value">{{ selectedOption?.label ?? "" }}</span>
      <svg class="ui-select__chevron" width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
        <path d="m4.5 6.25 3.5 3.5 3.5-3.5" />
      </svg>
    </button>

    <ul v-if="open" :id="listId" class="ui-select__menu" role="listbox">
      <li
        v-for="(option, index) in options"
        :id="`${listId}-option-${index}`"
        :key="option.value"
        class="ui-select__option"
        :class="{ 'ui-select__option--active': index === activeIndex }"
        role="option"
        :aria-selected="option.value === model"
        :aria-disabled="option.disabled || undefined"
        @pointermove="!option.disabled && (activeIndex = index)"
        @click="selectOption(option)"
      >
        <span class="ui-select__option-label">{{ option.label }}</span>
        <svg
          v-if="option.value === model"
          class="ui-select__check"
          width="16"
          height="16"
          viewBox="0 0 16 16"
          aria-hidden="true"
        >
          <path d="m3.5 8.25 2.75 2.75 6.25-6.25" />
        </svg>
      </li>
    </ul>
  </span>
</template>

<style scoped>
.ui-select {
  position: relative;
  display: inline-flex;
  width: 100%;
}

.ui-select__trigger {
  display: flex;
  width: 100%;
  height: var(--control-height);
  align-items: center;
  gap: var(--space-2);
  padding: 0 var(--space-2);
  color: var(--color-text-primary);
  border: 0.0625rem solid var(--color-border);
  border-radius: var(--radius-control);
  outline: none;
  background: var(--color-input);
  font: inherit;
  font-size: var(--font-size-sm);
  line-height: var(--line-height-sm);
  text-align: left;
  cursor: pointer;
}

.ui-select__trigger:focus,
.ui-select__trigger[aria-expanded="true"] {
  border-color: var(--color-focus);
}

.ui-select__trigger:disabled {
  cursor: not-allowed;
  opacity: 0.4;
}

.ui-select--pill {
  width: auto;
}

.ui-select--pill .ui-select__trigger {
  width: auto;
  height: var(--button-height-medium);
  gap: var(--space-3);
  padding: 0 0.875rem;
  border: none;
  border-radius: var(--radius-button-medium);
  background: var(--color-control);
}

.ui-select--pill .ui-select__trigger:hover:not(:disabled) {
  background: var(--color-interactive-hover);
}

.ui-select__value,
.ui-select__option-label {
  min-width: 0;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ui-select__chevron,
.ui-select__check {
  flex: none;
  fill: none;
  stroke: currentColor;
  stroke-width: 1.5;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.ui-select__chevron {
  color: var(--color-text-secondary);
}

.ui-select__menu {
  position: absolute;
  z-index: 100;
  top: calc(100% + 0.25rem);
  left: 0;
  display: flex;
  width: 100%;
  max-height: calc(100vh - 1.5rem);
  flex-direction: column;
  gap: 0;
  overflow-y: auto;
  margin: 0;
  padding: 0.25rem;
  list-style: none;
  border: 0.0625rem solid var(--color-border-inverted);
  border-radius: 0.75rem;
  background: var(--color-menu);
  box-shadow: var(--shadow-menu);
}

.ui-select--pill .ui-select__menu {
  right: 0;
  left: auto;
  width: max-content;
  min-width: 13.625rem;
  max-width: min(22.5rem, calc(100vw - 1.5rem));
}

.ui-select__option {
  display: flex;
  min-height: 2.5rem;
  align-items: center;
  gap: var(--space-2);
  padding: 0.5rem 0.625rem;
  color: var(--color-text-primary);
  border-radius: 0.625rem;
  font-size: var(--font-size-sm);
  line-height: var(--line-height-sm);
  cursor: pointer;
}

.ui-select__option--active:not([aria-disabled="true"]),
.ui-select__option:hover:not([aria-disabled="true"]) {
  background: var(--color-interactive-hover);
}

.ui-select__option[aria-disabled="true"] {
  cursor: not-allowed;
  opacity: 0.4;
}
</style>
