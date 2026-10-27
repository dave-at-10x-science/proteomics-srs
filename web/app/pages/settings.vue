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
  const body = {
    ...form.value,
    reviews_per_day: capReviews.value ? (form.value.reviews_per_day ?? 100) : null,
  };
  try {
    form.value = await api.saveSettings(body);
    message.value = 'Saved.';
  } catch (e: unknown) {
    const data = (e as { data?: { error?: string } }).data;
    message.value = data?.error ?? 'Could not save.';
  }
}

// Starting over takes two presses, so it can't happen by accident.
const confirmingReset = ref(false);
const resetMessage = ref('');
async function reset() {
  try {
    const { reviews_removed } = await api.resetProgress();
    try {
      sessionStorage.removeItem('srs.round');
    } catch {
      // No saved round to clear.
    }
    resetMessage.value = `Done. ${reviews_removed} ${reviews_removed === 1 ? 'review was' : 'reviews were'} cleared; everything is unstudied again.`;
  } catch {
    resetMessage.value = 'Could not reset. Is the server running?';
  } finally {
    confirmingReset.value = false;
  }
}
</script>

<template>
  <div class="stack">
    <form v-if="form" class="card stack" @submit.prevent="save">
      <h1>Settings</h1>
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

    <section class="card stack">
      <h2>Start over</h2>
      <p class="muted small">
        Forget everything you've studied and reviewed, including memory game results, as if you were starting for the
        first time. Your sets, notes and settings stay.
      </p>
      <template v-if="!confirmingReset">
        <button
          class="danger"
          @click="
            confirmingReset = true;
            resetMessage = '';
          "
        >
          Reset my progress
        </button>
      </template>
      <template v-else>
        <p><strong>This can't be undone.</strong> Reset all progress?</p>
        <div class="confirm">
          <button @click="confirmingReset = false">Cancel</button>
          <button class="danger solid" @click="reset">Yes, start over</button>
        </div>
      </template>
      <p v-if="resetMessage" class="muted">{{ resetMessage }}</p>
    </section>
  </div>
</template>

<style scoped>
h2 {
  margin: 0;
  font-size: 1.1rem;
}
.danger {
  border-color: var(--again);
  color: var(--again);
}
.danger.solid {
  background: var(--again);
  color: #fff;
}
.confirm {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.5rem;
}
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
