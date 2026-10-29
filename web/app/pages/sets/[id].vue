<script setup lang="ts">
import type { NoteType, SetSummary, StoredNote } from '~/types/api';
import ConfirmModal from '~/components/ConfirmModal.vue';

const route = useRoute();
const id = computed(() => String(route.params.id));
const api = useApi();
const confirm = useOverlay().create(ConfirmModal);
const set = ref<SetSummary | null>(null);
const notes = ref<StoredNote[]>([]);
const types = ref<Map<string, NoteType>>(new Map());
const filter = ref('');
const aiAvailable = ref(false);
const drafting = ref(false);
const publishing = ref(false);

onMounted(async () => {
  const [sets, ns, ts, ai] = await Promise.all([
    api.sets(),
    api.setNotes(id.value),
    api.noteTypes(),
    api.draftStatus().catch(() => ({ available: false })),
  ]);
  set.value = sets.find((s) => s.id === id.value) ?? null;
  notes.value = ns;
  types.value = new Map(ts.map((t) => [t.id, t]));
  aiAvailable.value = ai.available;
});

async function reload() {
  notes.value = await api.setNotes(id.value);
}

const drafts = computed(() => notes.value.filter((n) => n.status === 'draft'));

async function publishAll() {
  const n = drafts.value.length;
  const ok = await confirm.open({
    title: `Publish ${n} ${n === 1 ? 'draft' : 'drafts'}?`,
    description: 'Their cards join your daily reviews.',
    confirmLabel: 'Publish',
  }).result;
  if (!ok) return;
  publishing.value = true;
  try {
    await api.publishDrafts(id.value);
    await reload();
  } finally {
    publishing.value = false;
  }
}

const shown = computed(() => {
  const q = filter.value.trim().toLowerCase();
  return notes.value.filter((n) => !q || noteTitle(n, types.value.get(n.note_type_id)).toLowerCase().includes(q));
});
</script>

<template>
  <div class="flex flex-col gap-4">
    <UButton
      to="/library"
      label="Library"
      icon="i-lucide-arrow-left"
      color="neutral"
      variant="link"
      size="md"
      class="self-start px-0"
    />

    <template v-if="set">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div class="min-w-0">
          <h1 class="text-2xl font-bold tracking-tight text-highlighted">{{ set.name }}</h1>
          <p class="text-sm text-muted">{{ notes.length }} {{ notes.length === 1 ? 'note' : 'notes' }}</p>
        </div>
        <div class="flex gap-2">
          <UButton
            v-if="aiAvailable"
            label="Draft with AI"
            icon="i-lucide-sparkles"
            :color="drafting ? 'primary' : 'neutral'"
            :variant="drafting ? 'soft' : 'outline'"
            :aria-expanded="drafting"
            @click="drafting = !drafting"
          />
          <UButton :to="`/notes/new?set=${encodeURIComponent(set.id)}`" label="Add" icon="i-lucide-plus" />
        </div>
      </div>

      <UCard v-if="drafting">
        <DraftPanel :set-id="set.id" :note-types="[...types.values()]" :empty="!notes.length" @drafted="reload" />
      </UCard>

      <UAlert
        v-if="drafts.length"
        color="warning"
        variant="subtle"
        icon="i-lucide-file-pen-line"
        :title="`${drafts.length} ${drafts.length === 1 ? 'draft' : 'drafts'} to check`"
        description="Drafts are never studied until they're published."
        :actions="[{ label: 'Publish all', color: 'warning', size: 'md', loading: publishing, onClick: publishAll }]"
      />

      <UCard :ui="{ body: 'p-0 sm:p-0' }">
        <div v-if="notes.length > 8" class="border-b border-default p-3">
          <UInput v-model="filter" icon="i-lucide-search" placeholder="Filter" aria-label="Filter notes" class="w-full" />
        </div>
        <UEmpty
          v-if="!notes.length"
          icon="i-lucide-notebook-pen"
          title="No notes yet"
          :description="aiAvailable ? 'Add one, or draft a batch with AI.' : 'Add the first one.'"
          variant="naked"
        />
        <ul class="divide-y divide-default">
          <li v-for="n in shown" :key="n.id">
            <ULink
              :to="`/notes/${encodeURIComponent(n.id)}?set=${encodeURIComponent(set.id)}`"
              raw
              class="flex items-center gap-3 px-4 py-3 transition-colors hover:bg-elevated/50 sm:px-6"
            >
              <span class="min-w-0 flex-1 truncate text-highlighted">{{ noteTitle(n, types.get(n.note_type_id)) }}</span>
              <UBadge v-if="n.status === 'draft'" label="draft" color="warning" variant="soft" size="md" />
              <UIcon name="i-lucide-chevron-right" class="size-5 shrink-0 text-dimmed" />
            </ULink>
          </li>
        </ul>
      </UCard>
    </template>
    <p v-else class="text-muted">Loading…</p>
  </div>
</template>
