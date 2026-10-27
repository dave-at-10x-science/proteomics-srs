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
  <div class="study">
    <p v-if="error" class="card">{{ error }}</p>

    <section v-if="phase === 'setup'" class="card stack">
      <h2>Study new notes</h2>
      <template v-if="remaining !== null">
        <template v-if="remaining > 0">
          <p class="muted">
            <strong>{{ remaining }}</strong> {{ remaining === 1 ? 'note' : 'notes' }} not studied yet. Read through a
            batch; afterwards all of their cards join your review rounds.
          </p>
          <div class="sizes" role="radiogroup" aria-label="Notes in this batch">
            <button
              v-for="s in SIZES"
              :key="s"
              role="radio"
              :aria-checked="size === s"
              :class="{ selected: size === s }"
              @click="size = s"
            >
              {{ s }}
            </button>
          </div>
          <button class="primary wide" :disabled="busy" @click="start">
            Study {{ batchSize }} {{ batchSize === 1 ? 'note' : 'notes' }}
          </button>
        </template>
        <template v-else>
          <p>You've studied everything in your sets.</p>
          <NuxtLink to="/review"><button class="primary wide">Start a review round</button></NuxtLink>
        </template>
      </template>
      <p v-else class="muted">Loading…</p>
    </section>

    <template v-else-if="phase === 'studying'">
      <NoteFlipper v-model:index="index" :notes="notes" :types="types">
        <template #next="{ go }">
          <button v-if="atEnd" class="primary done" :disabled="busy" @click="finish">Done</button>
          <button v-else class="primary" aria-label="Next" @click="go(1)">→</button>
        </template>
      </NoteFlipper>
      <p class="muted small center">
        Take your time. When you press Done, these {{ notes.length }} go into your review rounds.
      </p>
    </template>

    <section v-else class="card stack center">
      <h2>Studied</h2>
      <p>
        {{ notes.length }} {{ notes.length === 1 ? 'note is' : 'notes are' }} now in your review rounds{{
          added < notes.length ? ` (${notes.length - added} already were)` : ''
        }}.
      </p>
      <div class="actions">
        <NuxtLink to="/review"><button class="primary wide">Start a review round</button></NuxtLink>
        <button class="wide" @click="loadSetup">Study more</button>
      </div>
    </section>
  </div>
</template>

<style scoped>
.study {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}
h2 {
  margin: 0;
  font-size: 1.1rem;
}
p {
  margin: 0;
}
.sizes {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 0.5rem;
}
.sizes .selected {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--focus);
  font-weight: 600;
}
.wide {
  width: 100%;
}
.done {
  font-size: 1rem !important;
  font-weight: 600;
}
.actions {
  display: grid;
  gap: 0.5rem;
}
.center {
  text-align: center;
}
.small {
  font-size: 0.85rem;
}
</style>
