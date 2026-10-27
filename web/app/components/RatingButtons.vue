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

const ratings: { id: Rating; label: string; key: string }[] = [
  { id: 'again', label: 'Again', key: '1' },
  { id: 'hard', label: 'Hard', key: '2' },
  { id: 'good', label: 'Good', key: '3' },
  { id: 'easy', label: 'Easy', key: '4' },
];

const shown = computed(() => ratings.filter((r) => !props.only || props.only.includes(r.id)));

// The suggested rating gets keyboard focus, so Enter takes it.
const buttons = ref<HTMLButtonElement[]>([]);
function focusSuggested() {
  const i = shown.value.findIndex((r) => r.id === props.suggested);
  if (i >= 0) buttons.value[i]?.focus();
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
  <div class="ratings" :style="{ gridTemplateColumns: `repeat(${shown.length}, 1fr)` }">
    <button
      v-for="r in shown"
      ref="buttons"
      :key="r.id"
      :class="[r.id, { suggested: suggested === r.id }]"
      :disabled="disabled"
      @click="emit('rate', r.id)"
    >
      <span class="name">{{ r.label }}</span>
      <span class="interval">{{ formatInterval(intervals[r.id]) }}</span>
    </button>
  </div>
</template>

<style scoped>
.ratings {
  display: grid;
  gap: 0.5rem;
}
button {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.1rem;
  padding: 0.7rem 0.25rem;
  border-width: 2px;
}
.name {
  font-weight: 600;
}
.interval {
  font-size: 0.8rem;
  color: var(--muted);
}
.again {
  border-color: var(--again);
}
.hard {
  border-color: var(--hard);
}
.good {
  border-color: var(--good);
}
.easy {
  border-color: var(--easy);
}
.suggested {
  box-shadow: 0 0 0 3px var(--focus);
}
.suggested.good {
  background: var(--good);
  color: #fff;
}
.suggested.good .interval {
  color: inherit;
  opacity: 0.85;
}
</style>
