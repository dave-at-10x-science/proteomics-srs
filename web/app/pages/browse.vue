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
  <div class="flex flex-col gap-4">
    <ErrorAlert :errors="error" />

    <div class="flex items-center gap-2">
      <USelect
        v-model="setId"
        :items="sets.map((s) => ({ label: s.name, value: s.id }))"
        aria-label="Set"
        class="min-w-0 flex-1"
      />
      <UButton
        icon="i-lucide-shuffle"
        :color="shuffled ? 'primary' : 'neutral'"
        :variant="shuffled ? 'soft' : 'outline'"
        :aria-pressed="shuffled"
        aria-label="Shuffle"
        @click="shuffled = !shuffled"
      />
    </div>

    <template v-if="notes.length">
      <NoteFlipper v-model:index="index" :notes="notes" :types="types" wrap />
      <p class="text-center text-sm text-muted">Swipe or use the arrow keys. Browsing doesn't change your schedule.</p>
    </template>
    <UCard v-else-if="setId">
      <UEmpty icon="i-lucide-layers" title="No cards in this set yet" variant="naked" />
    </UCard>
  </div>
</template>
