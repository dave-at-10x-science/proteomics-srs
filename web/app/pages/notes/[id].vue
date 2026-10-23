<script setup lang="ts">
import type { FieldValue, NoteType, StoredNote } from '~/types/api';

const route = useRoute();
const id = computed(() => String(route.params.id));
const back = computed(() => (route.query.set ? `/sets/${encodeURIComponent(String(route.query.set))}` : '/library'));
const api = useApi();
const note = ref<StoredNote | null>(null);
const noteType = ref<NoteType | null>(null);
const saving = ref(false);
const errors = ref<string[]>([]);
const saved = ref(false);

onMounted(async () => {
  note.value = await api.note(id.value);
  noteType.value = (await api.noteTypes()).find((t) => t.id === note.value!.note_type_id) ?? null;
});

async function save(fields: Record<string, FieldValue>, status: 'draft' | 'published') {
  saving.value = true;
  errors.value = [];
  saved.value = false;
  try {
    note.value = await api.updateNote(id.value, { fields, status });
    saved.value = true;
  } catch (e) {
    errors.value = apiErrors(e);
  } finally {
    saving.value = false;
  }
}

async function remove() {
  if (!confirm('Delete this note and its review history?')) return;
  await api.deleteNote(id.value);
  await navigateTo(back.value);
}
</script>

<template>
  <div class="stack">
    <NuxtLink :to="back" class="back">← Back</NuxtLink>
    <section v-if="note && noteType" class="card stack">
      <h1>{{ noteTitle(note, noteType) }}</h1>
      <NoteForm
        :note-type="noteType"
        :initial="noteFields(note)"
        :initial-status="note.status"
        :saving="saving"
        :errors="errors"
        @save="save"
      />
      <p v-if="saved" class="muted">Saved.</p>
    </section>
    <button v-if="note" class="delete" @click="remove">Delete note</button>
  </div>
</template>

<style scoped>
.back {
  text-decoration: none;
  color: var(--muted);
}
h1 {
  margin: 0;
  font-size: 1.1rem;
}
.delete {
  color: var(--again);
  border-color: var(--again);
  background: transparent;
}
</style>
