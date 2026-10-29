<script setup lang="ts">
import type { FieldValue, NoteType, StoredNote } from '~/types/api';
import ConfirmModal from '~/components/ConfirmModal.vue';

const route = useRoute();
const id = computed(() => String(route.params.id));
const back = computed(() => (route.query.set ? `/sets/${encodeURIComponent(String(route.query.set))}` : '/library'));
const api = useApi();
const toast = useToast();
const confirm = useOverlay().create(ConfirmModal);
const note = ref<StoredNote | null>(null);
const noteType = ref<NoteType | null>(null);
const saving = ref(false);
const errors = ref<string[]>([]);

onMounted(async () => {
  note.value = await api.note(id.value);
  noteType.value = (await api.noteTypes()).find((t) => t.id === note.value!.note_type_id) ?? null;
});

async function save(fields: Record<string, FieldValue>, status: 'draft' | 'published') {
  saving.value = true;
  errors.value = [];
  try {
    note.value = await api.updateNote(id.value, { fields, status });
    toast.add({ title: 'Saved', icon: 'i-lucide-circle-check', color: 'success' });
  } catch (e) {
    errors.value = apiErrors(e);
  } finally {
    saving.value = false;
  }
}

async function remove() {
  const ok = await confirm.open({
    title: 'Delete this note?',
    description: 'Its review history is deleted too. This can’t be undone.',
    confirmLabel: 'Delete',
    color: 'error',
  }).result;
  if (!ok) return;
  await api.deleteNote(id.value);
  await navigateTo(back.value);
}
</script>

<template>
  <div class="flex flex-col gap-4">
    <UButton
      :to="back"
      label="Back"
      icon="i-lucide-arrow-left"
      color="neutral"
      variant="link"
      size="md"
      class="self-start px-0"
    />
    <template v-if="note && noteType">
      <h1 class="text-2xl font-bold tracking-tight text-highlighted">{{ noteTitle(note, noteType) }}</h1>
      <UCard>
        <NoteForm
          :note-type="noteType"
          :initial="noteFields(note)"
          :initial-status="note.status"
          :saving="saving"
          :errors="errors"
          @save="save"
        />
      </UCard>
      <UButton label="Delete note" icon="i-lucide-trash-2" color="error" variant="soft" block @click="remove" />
    </template>
  </div>
</template>
