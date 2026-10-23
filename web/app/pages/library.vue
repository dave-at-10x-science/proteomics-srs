<script setup lang="ts">
import type { SetSummary } from '~/types/api';

const api = useApi();
const sets = ref<SetSummary[]>([]);
const name = ref('');
const errors = ref<string[]>([]);

async function load() {
  sets.value = await api.sets();
}

async function create() {
  if (!name.value.trim()) return;
  try {
    const set = await api.createSet(name.value.trim());
    await navigateTo(`/sets/${encodeURIComponent(set.id)}`);
  } catch (e) {
    errors.value = apiErrors(e);
  }
}

onMounted(load);
</script>

<template>
  <div class="stack">
    <section class="card">
      <h1>Library</h1>
      <ul class="list">
        <li v-for="s in sets" :key="s.id">
          <NuxtLink :to="`/sets/${encodeURIComponent(s.id)}`">
            <span>{{ s.name }}</span>
            <span class="muted small">{{ s.note_count }} notes{{ s.subscribed ? '' : ' · not studying' }}</span>
          </NuxtLink>
        </li>
      </ul>
    </section>

    <form class="card stack" @submit.prevent="create">
      <h2>New set</h2>
      <input v-model="name" type="text" placeholder="e.g. Proteases" />
      <p v-for="e in errors" :key="e" class="error">{{ e }}</p>
      <button class="primary" :disabled="!name.trim()">Create set</button>
    </form>
  </div>
</template>

<style scoped>
h1,
h2 {
  margin: 0 0 0.5rem;
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
  align-items: baseline;
  gap: 1rem;
  padding: 0.8rem 0;
  color: var(--text);
  text-decoration: none;
}
.small {
  font-size: 0.85rem;
}
.error {
  color: var(--again);
  margin: 0;
}
</style>
