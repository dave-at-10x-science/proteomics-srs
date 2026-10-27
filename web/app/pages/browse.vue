<script setup lang="ts">
import type { NoteType, SetSummary, StoredNote } from '~/types/api';

// Flash-card browsing: every field of each note, one note at a time. Nothing here is graded or
// touches the review schedule.
const api = useApi();
const route = useRoute();
const sets = ref<SetSummary[]>([]);
const types = ref<Map<string, NoteType>>(new Map());
const setId = ref('');
const notes = ref<StoredNote[]>([]);
const index = ref(0);
const shuffled = ref(false);
const error = ref('');

function shuffle<T>(xs: T[]): T[] {
  const out = [...xs];
  for (let i = out.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [out[i], out[j]] = [out[j]!, out[i]!];
  }
  return out;
}

async function loadSet() {
  if (!setId.value) return;
  const all = (await api.setNotes(setId.value)).filter((n) => n.status === 'published');
  notes.value = shuffled.value ? shuffle(all) : all;
  index.value = 0;
}

const note = computed(() => notes.value[index.value]);
const noteType = computed(() => (note.value ? types.value.get(note.value.note_type_id) : undefined));

function go(step: number) {
  const n = notes.value.length;
  if (n) index.value = (index.value + step + n) % n;
}

function onKey(e: KeyboardEvent) {
  if (e.target instanceof HTMLInputElement || e.target instanceof HTMLSelectElement) return;
  if (e.key === 'ArrowRight' || e.key === ' ') {
    e.preventDefault();
    go(1);
  } else if (e.key === 'ArrowLeft') {
    go(-1);
  }
}

// Swipe left or right on a phone.
let touchX: number | null = null;
function onTouchStart(e: TouchEvent) {
  touchX = e.touches[0]?.clientX ?? null;
}
function onTouchEnd(e: TouchEvent) {
  const end = e.changedTouches[0]?.clientX;
  if (touchX !== null && end !== undefined && Math.abs(end - touchX) > 50) go(end < touchX ? 1 : -1);
  touchX = null;
}

watch([setId, shuffled], loadSet);

onMounted(async () => {
  window.addEventListener('keydown', onKey);
  try {
    const [s, t] = await Promise.all([api.sets(), api.noteTypes()]);
    sets.value = s.filter((x) => x.note_count > 0);
    types.value = new Map(t.map((x) => [x.id, x]));
    const wanted = String(route.query.set ?? '');
    setId.value =
      sets.value.find((x) => x.id === wanted)?.id ?? sets.value.find((x) => x.subscribed)?.id ?? sets.value[0]?.id ?? '';
  } catch {
    error.value = 'Could not reach the server.';
  }
});
onBeforeUnmount(() => window.removeEventListener('keydown', onKey));
</script>

<template>
  <div class="browse">
    <p v-if="error" class="card">{{ error }}</p>

    <div class="controls">
      <select v-model="setId" aria-label="Set">
        <option v-for="s in sets" :key="s.id" :value="s.id">{{ s.name }}</option>
      </select>
      <label class="shuffle"><input v-model="shuffled" type="checkbox" /> Shuffle</label>
    </div>

    <template v-if="note && noteType">
      <section class="card stack" @touchstart.passive="onTouchStart" @touchend="onTouchEnd">
        <NoteFacts :note-type="noteType" :fields="noteFields(note)" visuals-first />
      </section>

      <div class="nav">
        <button aria-label="Previous" @click="go(-1)">←</button>
        <span class="muted">{{ index + 1 }} / {{ notes.length }}</span>
        <button class="primary" aria-label="Next" @click="go(1)">→</button>
      </div>
      <p class="muted small center">Swipe or use the arrow keys. Browsing doesn't change your schedule.</p>
    </template>
    <p v-else-if="setId" class="muted center">No cards in this set yet.</p>
  </div>
</template>

<style scoped>
.browse {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}
.controls {
  display: flex;
  gap: 0.75rem;
  align-items: center;
}
select {
  flex: 1;
  min-width: 0;
  font: inherit;
  color: inherit;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 0.6rem 0.75rem;
  min-height: 44px;
}
.shuffle {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  white-space: nowrap;
}
.nav {
  display: grid;
  grid-template-columns: 1fr auto 1fr;
  align-items: center;
  gap: 0.75rem;
  text-align: center;
  font-variant-numeric: tabular-nums;
}
.nav button {
  font-size: 1.25rem;
}
.center {
  text-align: center;
}
.small {
  font-size: 0.85rem;
  margin: 0;
}
</style>
