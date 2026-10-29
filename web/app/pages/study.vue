<script setup lang="ts">
import type { NoteType, StoredNote } from '~/types/api';

// Study a batch of notes not seen before, with every field shown. Finishing the batch puts all
// of their cards into review rounds.
const api = useApi();
const SIZE_KEY = 'srs.studySize';
const SIZES = [3, 5, 10, 20];

const phase = ref<'setup' | 'studying' | 'finished'>('setup');
const remaining = ref<number | null>(null);
const size = ref(5);
const notes = ref<StoredNote[]>([]);
const types = ref<Map<string, NoteType>>(new Map());
const index = ref(0);
const added = ref(0);
const busy = ref(false);
const error = ref('');

const batchSize = computed(() => Math.min(size.value, remaining.value ?? 0));
const atEnd = computed(() => index.value === notes.value.length - 1);

async function loadSetup() {
  phase.value = 'setup';
  try {
    remaining.value = (await api.studyBatch(1)).remaining;
    error.value = '';
  } catch {
    error.value = 'Could not reach the server.';
  }
}

async function start() {
  busy.value = true;
  try {
    try {
      localStorage.setItem(SIZE_KEY, String(size.value));
    } catch {
      // Not remembering the size is fine.
    }
    const [batch, t] = await Promise.all([api.studyBatch(size.value), api.noteTypes()]);
    types.value = new Map(t.map((x) => [x.id, x]));
    notes.value = batch.notes;
    index.value = 0;
    phase.value = 'studying';
  } catch {
    error.value = 'Could not load notes to study.';
  } finally {
    busy.value = false;
  }
}

async function finish() {
  busy.value = true;
  try {
    added.value = (await api.markStudied(notes.value.map((n) => n.id))).added;
    phase.value = 'finished';
  } catch {
    error.value = 'Could not save. Try again.';
  } finally {
    busy.value = false;
  }
}

onMounted(() => {
  try {
    const saved = Number(localStorage.getItem(SIZE_KEY));
    if (saved > 0) size.value = saved;
  } catch {
    // Use the default size.
  }
  loadSetup();
});
</script>

<template>
  <div class="flex flex-col gap-4">
    <ErrorAlert :errors="error" />

    <template v-if="phase === 'setup'">
      <UCard v-if="remaining === null">
        <p class="text-muted">Loading…</p>
      </UCard>
      <UCard v-else-if="remaining > 0" :ui="{ body: 'flex flex-col gap-5' }">
        <div class="flex items-center gap-3">
          <span class="flex size-11 items-center justify-center rounded-xl bg-primary/12 text-primary">
            <UIcon name="i-lucide-book-open" class="size-6" />
          </span>
          <div>
            <h1 class="text-lg font-semibold text-highlighted">Study new notes</h1>
            <p class="text-sm text-muted">
              <span class="font-semibold text-default tabular-nums">{{ remaining }}</span>
              {{ remaining === 1 ? 'note' : 'notes' }} not studied yet
            </p>
          </div>
        </div>
        <p class="text-sm text-toned">Read through a batch. Afterwards, all of their cards join your review rounds.</p>
        <SegmentedPicker v-model="size" :items="SIZES" legend="Notes in this batch" />
        <UButton
          :label="`Study ${batchSize} ${batchSize === 1 ? 'note' : 'notes'}`"
          trailing-icon="i-lucide-arrow-right"
          block
          :loading="busy"
          @click="start"
        />
      </UCard>
      <UCard v-else>
        <UEmpty
          icon="i-lucide-graduation-cap"
          title="You've studied everything"
          description="Every note in your sets is in your review rounds."
          variant="naked"
          :actions="[{ label: 'Start a review round', icon: 'i-lucide-play', to: '/review' }]"
        />
      </UCard>
    </template>

    <template v-else-if="phase === 'studying'">
      <NoteFlipper v-model:index="index" :notes="notes" :types="types">
        <template #next="{ go }">
          <UButton v-if="atEnd" label="Done" icon="i-lucide-check" block :loading="busy" @click="finish" />
          <UButton v-else icon="i-lucide-arrow-right" aria-label="Next" block @click="go(1)" />
        </template>
      </NoteFlipper>
      <p class="text-center text-sm text-muted">
        Take your time. When you press Done, these {{ notes.length }} go into your review rounds.
      </p>
    </template>

    <UCard v-else :ui="{ body: 'flex flex-col items-center gap-4 py-8 text-center' }">
      <span class="flex size-16 items-center justify-center rounded-full bg-success/12 text-success">
        <UIcon name="i-lucide-graduation-cap" class="size-8" />
      </span>
      <div>
        <h1 class="text-xl font-semibold text-highlighted">Studied</h1>
        <p class="mt-1 text-muted">
          {{ notes.length }} {{ notes.length === 1 ? 'note is' : 'notes are' }} now in your review rounds{{
            added < notes.length ? ` (${notes.length - added} already were)` : ''
          }}.
        </p>
      </div>
      <div class="grid w-full gap-2">
        <UButton label="Start a review round" icon="i-lucide-play" to="/review" block />
        <UButton label="Study more" icon="i-lucide-book-open" color="neutral" variant="outline" block @click="loadSetup" />
      </div>
    </UCard>
  </div>
</template>
