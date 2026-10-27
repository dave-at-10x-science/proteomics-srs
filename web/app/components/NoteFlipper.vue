<script setup lang="ts">
import type { NoteType, StoredNote } from '~/types/api';

/** Flip through notes one at a time with every field shown. Arrow keys and swipes move. */
const props = defineProps<{ notes: StoredNote[]; types: Map<string, NoteType>; wrap?: boolean }>();
const index = defineModel<number>('index', { required: true });

const note = computed(() => props.notes[index.value]);
const noteType = computed(() => (note.value ? props.types.get(note.value.note_type_id) : undefined));

function go(step: number) {
  const n = props.notes.length;
  if (!n) return;
  const to = index.value + step;
  index.value = props.wrap ? (to + n) % n : Math.min(Math.max(to, 0), n - 1);
}

function onKey(e: KeyboardEvent) {
  if (e.target instanceof HTMLInputElement || e.target instanceof HTMLSelectElement) return;
  if (e.key === 'ArrowRight') go(1);
  else if (e.key === 'ArrowLeft') go(-1);
}

let touchX: number | null = null;
function onTouchStart(e: TouchEvent) {
  touchX = e.touches[0]?.clientX ?? null;
}
function onTouchEnd(e: TouchEvent) {
  const end = e.changedTouches[0]?.clientX;
  if (touchX !== null && end !== undefined && Math.abs(end - touchX) > 50) go(end < touchX ? 1 : -1);
  touchX = null;
}

onMounted(() => window.addEventListener('keydown', onKey));
onBeforeUnmount(() => window.removeEventListener('keydown', onKey));
</script>

<template>
  <div v-if="note && noteType" class="flipper">
    <section class="card stack" @touchstart.passive="onTouchStart" @touchend="onTouchEnd">
      <NoteFacts :note-type="noteType" :fields="noteFields(note)" visuals-first />
    </section>
    <div class="nav">
      <button aria-label="Previous" :disabled="!wrap && index === 0" @click="go(-1)">←</button>
      <span class="muted">{{ index + 1 }} / {{ notes.length }}</span>
      <slot name="next" :go="go">
        <button class="primary" aria-label="Next" @click="go(1)">→</button>
      </slot>
    </div>
  </div>
</template>

<style scoped>
.flipper {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}
.nav {
  display: grid;
  grid-template-columns: 1fr auto 1fr;
  align-items: center;
  gap: 0.75rem;
  text-align: center;
  font-variant-numeric: tabular-nums;
}
.nav :deep(button) {
  font-size: 1.25rem;
}
</style>
