<script setup lang="ts">
import type { Settings } from '~/types/api';
import ConfirmModal from '~/components/ConfirmModal.vue';

const api = useApi();
const toast = useToast();
const confirm = useOverlay().create(ConfirmModal);
const form = ref<Settings | null>(null);
const capReviews = ref(false);
const errors = ref<string[]>([]);

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
    errors.value = [];
    toast.add({ title: 'Saved', icon: 'i-lucide-circle-check', color: 'success' });
  } catch (e) {
    errors.value = apiErrors(e);
  }
}

// Starting over asks first, so it can't happen by accident.
const resetError = ref('');
async function reset() {
  const ok = await confirm.open({
    title: 'Start over?',
    description:
      'This forgets everything you’ve studied and reviewed, including memory game results. It can’t be undone.',
    confirmLabel: 'Yes, start over',
    color: 'error',
  }).result;
  if (!ok) return;
  try {
    const { reviews_removed } = await api.resetProgress();
    try {
      sessionStorage.removeItem('srs.round');
    } catch {
      // No saved round to clear.
    }
    resetError.value = '';
    toast.add({
      title: 'Progress reset',
      description: `${reviews_removed} ${reviews_removed === 1 ? 'review was' : 'reviews were'} cleared; everything is unstudied again.`,
      icon: 'i-lucide-rotate-ccw',
    });
  } catch {
    resetError.value = 'Could not reset. Is the server running?';
  }
}
</script>

<template>
  <div class="flex flex-col gap-4">
    <h1 class="text-2xl font-bold tracking-tight text-highlighted">Settings</h1>

    <UCard v-if="form">
      <template #header>
        <h2 class="font-semibold text-highlighted">Reviews</h2>
      </template>
      <form class="flex flex-col gap-5" @submit.prevent="save">
        <USwitch v-model="capReviews" label="Limit reviews per day" />
        <UFormField v-if="capReviews" label="Reviews per day">
          <UInputNumber v-model="form.reviews_per_day" :min="0" class="w-full" />
        </UFormField>
        <UFormField
          label="Target retention"
          :hint="`${Math.round(form.desired_retention * 100)}%`"
          description="Higher means more frequent reviews and fewer lapses."
          :ui="{ hint: 'font-semibold text-highlighted tabular-nums' }"
        >
          <USlider v-model="form.desired_retention" :min="0.7" :max="0.99" :step="0.01" class="py-2" />
        </UFormField>
        <ErrorAlert :errors="errors" />
        <UButton type="submit" label="Save" icon="i-lucide-check" block />
      </form>
    </UCard>

    <UCard>
      <template #header>
        <h2 class="font-semibold text-highlighted">Appearance</h2>
      </template>
      <UFormField label="Theme">
        <UColorModeSelect class="w-full" />
      </UFormField>
    </UCard>

    <UCard :ui="{ body: 'flex flex-col gap-4' }">
      <template #header>
        <h2 class="font-semibold text-highlighted">Start over</h2>
      </template>
      <p class="text-sm text-toned">
        Forget everything you've studied and reviewed, including memory game results, as if you were starting for the
        first time. Your sets, notes and settings stay.
      </p>
      <ErrorAlert :errors="resetError" />
      <UButton label="Reset my progress" icon="i-lucide-rotate-ccw" color="error" variant="soft" block @click="reset" />
    </UCard>
  </div>
</template>
