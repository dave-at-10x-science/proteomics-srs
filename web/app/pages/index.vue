<script setup lang="ts">
import type { Counts, SetSummary } from '~/types/api';

const api = useApi();
const counts = ref<Counts | null>(null);
const sets = ref<SetSummary[]>([]);
const error = ref('');

async function load() {
  try {
    let round;
    [round, sets.value] = await Promise.all([api.round(1), api.sets()]);
    counts.value = round.counts;
    error.value = '';
  } catch {
    error.value = 'Could not reach the server. Is srs-server running?';
  }
}

async function toggle(set: SetSummary) {
  await api.subscribe(set.id, !set.subscribed);
  await load();
}

const total = computed(() => {
  const c = counts.value;
  return c ? c.learning + c.review + c.new : 0;
});

onMounted(load);
</script>

<template>
  <div class="stack">
    <p v-if="error" class="card error">{{ error }}</p>

    <section v-if="counts" class="card today">
      <h1>Today</h1>
      <div class="counts">
        <div><strong>{{ counts.review }}</strong><span>to review</span></div>
        <div><strong>{{ counts.new }}</strong><span>new</span></div>
        <div><strong>{{ counts.learning }}</strong><span>learning</span></div>
      </div>
      <div class="actions">
        <NuxtLink v-if="total > 0" to="/review">
          <button class="primary start">Start a round</button>
        </NuxtLink>
        <p v-else class="muted done">All done for today. Come back tomorrow.</p>
        <NuxtLink to="/browse"><button class="start">Browse cards</button></NuxtLink>
      </div>
    </section>

    <section v-if="sets.length" class="card">
      <h2>Sets</h2>
      <ul class="sets">
        <li v-for="s in sets" :key="s.id">
          <label>
            <input type="checkbox" :checked="s.subscribed" @change="toggle(s)" />
            <span>
              <span class="set-name">{{ s.name }}</span>
              <span class="muted small">{{ s.note_count }} notes · {{ s.card_count }} cards</span>
            </span>
          </label>
        </li>
      </ul>
    </section>
  </div>
</template>

<style scoped>
h1,
h2 {
  margin: 0 0 0.75rem;
  font-size: 1.1rem;
}
.counts {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  text-align: center;
  margin-bottom: 1rem;
}
.counts strong {
  display: block;
  font-size: 2rem;
}
.counts span {
  color: var(--muted);
  font-size: 0.85rem;
}
.start {
  width: 100%;
}
.actions {
  display: grid;
  gap: 0.5rem;
}
.done {
  text-align: center;
  margin: 0;
}
.error {
  border-color: var(--again);
}
.sets {
  list-style: none;
  margin: 0;
  padding: 0;
}
.sets li + li {
  border-top: 1px solid var(--border);
}
.sets label {
  display: flex;
  gap: 0.75rem;
  align-items: center;
  padding: 0.6rem 0;
  cursor: pointer;
}
.sets input {
  width: 1.3rem;
  height: 1.3rem;
}
.set-name {
  display: block;
}
.small {
  font-size: 0.85rem;
}
</style>
