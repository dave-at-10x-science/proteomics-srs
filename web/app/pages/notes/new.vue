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
  <div class="flex flex-col gap-4">
    <UButton
      :to="`/sets/${encodeURIComponent(setId)}`"
      label="Back to set"
      icon="i-lucide-arrow-left"
      color="neutral"
      variant="link"
      size="md"
      class="self-start px-0"
    />
    <h1 class="text-2xl font-bold tracking-tight text-highlighted">New note</h1>
    <UCard :ui="{ body: 'flex flex-col gap-5' }">
      <UFormField v-if="types.length > 1" label="Kind">
        <USelect v-model="typeId" :items="types.map((t) => t.id)" class="w-full" />
      </UFormField>
      <NoteForm v-if="noteType" :key="noteType.id" :note-type="noteType" :saving="saving" :errors="errors" @save="save" />
    </UCard>
  </div>
</template>
