<script setup lang="ts">
import type { Settings } from '~/types/api';

const api = useApi();
const form = ref<Settings | null>(null);
const capReviews = ref(false);
const message = ref('');

onMounted(async () => {
  form.value = await api.settings();
  capReviews.value = form.value.reviews_per_day !== null;
});

async function save() {
  if (!form.value) return;
  const body = { ...form.value, reviews_per_day: capReviews.value ? (form.value.reviews_per_day ?? 100) : null };
  try {
    form.value = await api.saveSettings(body);
    message.value = 'Saved.';
  } catch (e: unknown) {
    const data = (e as { data?: { error?: string } }).data;
    message.value = data?.error ?? 'Could not save.';
  }
}
</script>

<template>
  <form v-if="form" class="card stack" @submit.prevent="save">
    <h1>Settings</h1>
    <label>
      New cards per day
      <input v-model.number="form.new_per_day" type="number" min="0" max="200" />
    </label>
    <label class="row">
      <input v-model="capReviews" type="checkbox" />
      Limit reviews per day
    </label>
    <label v-if="capReviews">
      Reviews per day
      <input v-model.number="form.reviews_per_day" type="number" min="0" />
    </label>
    <label>
      Target retention: {{ Math.round(form.desired_retention * 100) }}%
      <input v-model.number="form.desired_retention" type="range" min="0.7" max="0.99" step="0.01" />
      <span class="muted small">Higher means more frequent reviews and fewer lapses.</span>
    </label>
    <button class="primary">Save</button>
    <p v-if="message" class="muted">{{ message }}</p>
  </form>
</template>

<style scoped>
h1 {
  margin: 0;
  font-size: 1.1rem;
}
label {
  display: grid;
  gap: 0.35rem;
}
.row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}
.row input {
  width: 1.2rem;
  height: 1.2rem;
}
input[type='range'] {
  width: 100%;
}
.small {
  font-size: 0.85rem;
}
</style>
