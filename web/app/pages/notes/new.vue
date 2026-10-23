<script setup lang="ts">
import type { FieldValue, NoteType } from '~/types/api';

const route = useRoute();
const setId = computed(() => String(route.query.set ?? ''));
const api = useApi();
const types = ref<NoteType[]>([]);
const typeId = ref('');
const saving = ref(false);
const errors = ref<string[]>([]);

onMounted(async () => {
  const [ts, existing] = await Promise.all([api.noteTypes(), setId.value ? api.setNotes(setId.value) : []]);
  types.value = ts;
  // Default to the type the set already uses.
  typeId.value = existing[0]?.note_type_id ?? ts[0]?.id ?? '';
});

const noteType = computed(() => types.value.find((t) => t.id === typeId.value));

async function save(fields: Record<string, FieldValue>, status: 'draft' | 'published') {
  saving.value = true;
  errors.value = [];
  try {
    await api.createNote({ set_id: setId.value, note_type_id: typeId.value, fields, status });
    await navigateTo(`/sets/${encodeURIComponent(setId.value)}`);
  } catch (e) {
    errors.value = apiErrors(e);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <div class="stack">
    <NuxtLink :to="`/sets/${encodeURIComponent(setId)}`" class="back">← Back to set</NuxtLink>
    <section class="card stack">
      <h1>New note</h1>
      <label v-if="types.length > 1" class="type">
        Kind
        <select v-model="typeId">
          <option v-for="t in types" :key="t.id" :value="t.id">{{ t.id }}</option>
        </select>
      </label>
      <NoteForm v-if="noteType" :key="noteType.id" :note-type="noteType" :saving="saving" :errors="errors" @save="save" />
    </section>
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
.type {
  display: grid;
  gap: 0.35rem;
  font-weight: 600;
}
select {
  font: inherit;
  padding: 0.6rem;
  border-radius: var(--radius);
  border: 1px solid var(--border);
  background: var(--surface);
  color: var(--text);
  min-height: 44px;
}
</style>
