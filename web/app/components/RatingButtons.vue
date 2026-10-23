<script setup lang="ts">
import type { Rating } from '~/types/api';

defineProps<{ intervals: Record<Rating, number>; suggested?: Rating; disabled?: boolean }>();
const emit = defineEmits<{ rate: [Rating] }>();

const ratings: { id: Rating; label: string; key: string }[] = [
  { id: 'again', label: 'Again', key: '1' },
  { id: 'hard', label: 'Hard', key: '2' },
  { id: 'good', label: 'Good', key: '3' },
  { id: 'easy', label: 'Easy', key: '4' },
];

// Keyboard shortcuts 1-4 on desktop.
function onKey(e: KeyboardEvent) {
  if (e.target instanceof HTMLInputElement) return;
  const r = ratings.find((r) => r.key === e.key);
  if (r) emit('rate', r.id);
}
onMounted(() => window.addEventListener('keydown', onKey));
onBeforeUnmount(() => window.removeEventListener('keydown', onKey));
</script>

<template>
  <div class="ratings">
    <button
      v-for="r in ratings"
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
  grid-template-columns: repeat(4, 1fr);
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
</style>
