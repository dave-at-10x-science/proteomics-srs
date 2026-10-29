<script setup lang="ts">
import type { Counts, SetSummary } from '~/types/api';

const api = useApi();
const counts = ref<Counts | null>(null);
const unstudied = ref(0);
const sets = ref<SetSummary[]>([]);
const error = ref('');

async function load() {
  try {
    let round;
    [round, sets.value] = await Promise.all([api.round(1), api.sets()]);
    counts.value = round.counts;
    unstudied.value = round.unstudied;
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
  <div class="flex flex-col gap-4">
    <ErrorAlert :errors="error" />

    <!-- Today: what's due and the next thing to do. -->
    <section
      v-if="counts"
      class="relative overflow-hidden rounded-2xl bg-linear-to-br from-primary-600 to-primary-900 p-5 text-white shadow-lg shadow-primary-900/20"
    >
      <!-- Decoration echoing the app icon. -->
      <svg viewBox="0 0 24 24" class="pointer-events-none absolute -top-10 -right-14 size-48 text-white/8" aria-hidden="true">
        <path d="M12 2.5 20.2 7.25v9.5L12 21.5l-8.2-4.75v-9.5Z" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round" />
        <circle cx="12" cy="12" r="1.6" fill="currentColor" />
      </svg>

      <p class="text-sm font-medium tracking-wide text-white/75 uppercase">Today</p>
      <div class="mt-3 flex items-end gap-6">
        <div>
          <div class="text-5xl leading-none font-bold tracking-tight tabular-nums">{{ total }}</div>
          <div class="mt-1 text-sm text-white/80">due for review</div>
        </div>
        <div>
          <div class="text-2xl leading-none font-semibold tabular-nums">{{ unstudied }}</div>
          <div class="mt-1 text-sm text-white/80">not studied yet</div>
        </div>
      </div>

      <div class="mt-5 flex flex-col gap-2">
        <UButton
          v-if="total > 0"
          to="/review"
          icon="i-lucide-play"
          label="Start a review round"
          block
          class="bg-white text-primary-800 hover:bg-primary-50 active:bg-primary-100"
        />
        <p v-else class="flex items-center gap-2 text-white/90">
          <UIcon name="i-lucide-circle-check" class="size-5" /> Nothing due for review right now.
        </p>
        <UButton
          v-if="unstudied > 0"
          to="/study"
          icon="i-lucide-book-open"
          label="Study new notes"
          block
          :class="
            total === 0
              ? 'bg-white text-primary-800 hover:bg-primary-50 active:bg-primary-100'
              : 'bg-white/12 text-white ring ring-white/25 ring-inset hover:bg-white/20 active:bg-white/25'
          "
        />
      </div>
    </section>

    <UCard v-if="sets.length" :ui="{ body: 'p-0 sm:p-0' }">
      <template #header>
        <h2 class="font-semibold text-highlighted">Your sets</h2>
        <p class="text-sm text-muted">Switch on the sets you're studying.</p>
      </template>
      <ul class="divide-y divide-default">
        <li v-for="s in sets" :key="s.id">
          <USwitch
            :model-value="s.subscribed"
            :label="s.name"
            :description="`${s.note_count} notes · ${s.card_count} cards`"
            class="w-full px-4 py-3 sm:px-6"
            :ui="{ root: 'flex-row-reverse justify-between gap-4', wrapper: 'ms-0', label: 'font-medium' }"
            @update:model-value="toggle(s)"
          />
        </li>
      </ul>
    </UCard>
  </div>
</template>
