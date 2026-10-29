<script setup lang="ts">
import type { Rating } from '~/types/api';

const props = defineProps<{
  intervals: Record<Rating, number>;
  suggested?: Rating;
  disabled?: boolean;
  /** Ratings to offer; all four by default. */
  only?: Rating[];
}>();
const emit = defineEmits<{ rate: [Rating] }>();

const ratings = [
  { id: 'again', label: 'Again', key: '1', color: 'error' },
  { id: 'hard', label: 'Hard', key: '2', color: 'warning' },
  { id: 'good', label: 'Good', key: '3', color: 'success' },
  { id: 'easy', label: 'Easy', key: '4', color: 'info' },
] as const;

const shown = computed(() => ratings.filter((r) => !props.only || props.only.includes(r.id)));

// The suggested rating gets keyboard focus, so Enter takes it.
const root = useTemplateRef<HTMLElement>('root');
function focusSuggested() {
  root.value?.querySelector<HTMLElement>(`[data-rating="${props.suggested}"]`)?.focus();
}

// Keyboard shortcuts 1-4 on desktop.
function onKey(e: KeyboardEvent) {
  if (e.target instanceof HTMLInputElement) return;
  const r = shown.value.find((r) => r.key === e.key);
  if (r) emit('rate', r.id);
}
onMounted(() => {
  window.addEventListener('keydown', onKey);
  nextTick(focusSuggested);
});
onBeforeUnmount(() => window.removeEventListener('keydown', onKey));
</script>

<template>
  <div ref="root" class="grid gap-2" :style="{ gridTemplateColumns: `repeat(${shown.length}, 1fr)` }">
    <UButton
      v-for="r in shown"
      :key="r.id"
      :data-rating="r.id"
      :color="r.color"
      :variant="suggested === r.id ? 'solid' : 'soft'"
      :disabled="disabled"
      class="flex-col justify-center gap-0 px-1 py-2.5"
      @click="emit('rate', r.id)"
    >
      <span class="font-semibold">{{ r.label }}</span>
      <span class="text-xs opacity-80">{{ formatInterval(intervals[r.id]) }}</span>
    </UButton>
  </div>
</template>
