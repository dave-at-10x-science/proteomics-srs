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

async function loadSet() {
  if (!setId.value) return;
  const all = (await api.setNotes(setId.value)).filter((n) => n.status === 'published');
  notes.value = shuffled.value ? shuffle(all) : all;
  index.value = 0;
}

watch([setId, shuffled], loadSet);

onMounted(async () => {
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

    <template v-if="notes.length">
      <NoteFlipper v-model:index="index" :notes="notes" :types="types" wrap />
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
.center {
  text-align: center;
}
.small {
  font-size: 0.85rem;
  margin: 0;
}
</style>
