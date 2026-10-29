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
  <div class="flex flex-col gap-4">
    <h1 class="text-2xl font-bold tracking-tight text-highlighted">Library</h1>

    <UCard :ui="{ body: 'p-0 sm:p-0' }">
      <ul class="divide-y divide-default">
        <li v-for="s in sets" :key="s.id">
          <ULink
            :to="`/sets/${encodeURIComponent(s.id)}`"
            raw
            class="flex items-center gap-3 px-4 py-3 transition-colors hover:bg-elevated/50 sm:px-6"
          >
            <span
              class="flex size-10 shrink-0 items-center justify-center rounded-lg"
              :class="s.subscribed ? 'bg-primary/12 text-primary' : 'bg-elevated text-muted'"
            >
              <UIcon name="i-lucide-layers" class="size-5" />
            </span>
            <span class="min-w-0 flex-1">
              <span class="block font-medium text-highlighted">{{ s.name }}</span>
              <span class="block text-sm text-muted">
                {{ s.note_count }} notes{{ s.subscribed ? '' : ' · not studying' }}
              </span>
            </span>
            <UIcon name="i-lucide-chevron-right" class="size-5 shrink-0 text-dimmed" />
          </ULink>
        </li>
      </ul>
    </UCard>

    <UCard>
      <form class="flex flex-col gap-3" @submit.prevent="create">
        <UFormField label="New set" description="Make your own set, then add notes to it.">
          <UInput v-model="name" placeholder="e.g. Proteases" class="w-full" />
        </UFormField>
        <ErrorAlert :errors="errors" />
        <UButton type="submit" label="Create set" icon="i-lucide-plus" block :disabled="!name.trim()" />
      </form>
    </UCard>
  </div>
</template>
