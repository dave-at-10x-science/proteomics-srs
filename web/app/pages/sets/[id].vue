<script setup lang="ts">
import type { NoteType, SetSummary, StoredNote } from '~/types/api';

const route = useRoute();
const id = computed(() => String(route.params.id));
const api = useApi();
const set = ref<SetSummary | null>(null);
const notes = ref<StoredNote[]>([]);
const types = ref<Map<string, NoteType>>(new Map());
const filter = ref('');

onMounted(async () => {
  const [sets, ns, ts] = await Promise.all([api.sets(), api.setNotes(id.value), api.noteTypes()]);
  set.value = sets.find((s) => s.id === id.value) ?? null;
  notes.value = ns;
  types.value = new Map(ts.map((t) => [t.id, t]));
});

const shown = computed(() => {
  const q = filter.value.trim().toLowerCase();
  return notes.value.filter((n) => !q || noteTitle(n, types.value.get(n.note_type_id)).toLowerCase().includes(q));
});
</script>

<template>
  <div class="stack">
    <NuxtLink to="/library" class="back">← Library</NuxtLink>
    <section v-if="set" class="card stack">
      <div class="head">
        <h1>{{ set.name }}</h1>
        <NuxtLink :to="`/notes/new?set=${encodeURIComponent(set.id)}`"><button class="primary">Add</button></NuxtLink>
      </div>
      <input v-if="notes.length > 8" v-model="filter" type="text" placeholder="Filter" />
      <p v-if="!notes.length" class="muted">No notes yet.</p>
      <ul class="list">
        <li v-for="n in shown" :key="n.id">
          <NuxtLink :to="`/notes/${encodeURIComponent(n.id)}?set=${encodeURIComponent(set.id)}`">
            <span>{{ noteTitle(n, types.get(n.note_type_id)) }}</span>
            <span v-if="n.status === 'draft'" class="badge">draft</span>
          </NuxtLink>
        </li>
      </ul>
    </section>
    <p v-else class="muted">Loading…</p>
  </div>
</template>

<style scoped>
.back {
  text-decoration: none;
  color: var(--muted);
}
.head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 1rem;
}
h1 {
  margin: 0;
  font-size: 1.1rem;
}
.list {
  list-style: none;
  margin: 0;
  padding: 0;
}
.list li + li {
  border-top: 1px solid var(--border);
}
.list a {
  display: flex;
  justify-content: space-between;
  padding: 0.75rem 0;
  color: var(--text);
  text-decoration: none;
}
.badge {
  font-size: 0.75rem;
  border: 1px solid var(--hard);
  color: var(--hard);
  border-radius: 999px;
  padding: 0 0.5rem;
}
</style>
