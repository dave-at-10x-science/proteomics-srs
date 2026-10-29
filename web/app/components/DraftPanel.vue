<script setup lang="ts">
import type { DraftResult, NoteType } from '~/types/api';

const props = defineProps<{
  setId: string;
  /** Note types to choose from when the set is empty; otherwise the set's own type is used. */
  noteTypes: NoteType[];
  empty: boolean;
}>();
const emit = defineEmits<{ drafted: [] }>();

const api = useApi();
const source = ref('');
const count = ref(10);
const noteTypeId = ref('');
const busy = ref(false);
const errors = ref<string[]>([]);
const result = ref<DraftResult | null>(null);

watchEffect(() => {
  if (!noteTypeId.value && props.noteTypes.length) noteTypeId.value = props.noteTypes[0]!.id;
});

const canDraft = computed(() => !busy.value && source.value.trim() && count.value >= 1 && count.value <= 30);

async function submit() {
  busy.value = true;
  errors.value = [];
  result.value = null;
  try {
    result.value = await api.draft(props.setId, {
      source: source.value,
      count: count.value,
      note_type_id: props.empty ? noteTypeId.value : undefined,
    });
    if (result.value.created.length) {
      source.value = '';
      emit('drafted');
    }
  } catch (e) {
    errors.value = apiErrors(e);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <form class="flex flex-col gap-4" @submit.prevent="submit">
    <UFormField label="What should the cards cover?">
      <UTextarea
        v-model="source"
        :rows="4"
        autoresize
        :maxrows="12"
        placeholder="A topic (“proteases used in sample prep”), a list of terms, or text pasted from a paper"
        class="w-full"
      />
    </UFormField>
    <div class="flex gap-3">
      <UFormField label="How many" class="w-28 shrink-0">
        <UInputNumber v-model="count" :min="1" :max="30" class="w-full" />
      </UFormField>
      <UFormField v-if="empty" label="Card type" class="min-w-0 flex-1">
        <USelect v-model="noteTypeId" :items="noteTypes.map((t) => t.id)" class="w-full" />
      </UFormField>
    </div>
    <UButton
      type="submit"
      :label="busy ? 'Drafting… this can take a minute' : 'Draft'"
      icon="i-lucide-sparkles"
      block
      :loading="busy"
      :disabled="!canDraft"
    />
    <ErrorAlert :errors="errors" />
    <UAlert
      v-if="result"
      :color="result.created.length ? 'success' : 'neutral'"
      variant="subtle"
      :icon="result.created.length ? 'i-lucide-circle-check' : 'i-lucide-info'"
      :title="`${result.created.length} ${result.created.length === 1 ? 'draft' : 'drafts'} added`"
      description="Check each one before publishing; drafts are never studied."
    />
    <UCollapsible v-if="result?.skipped.length" class="flex flex-col gap-2">
      <UButton
        :label="`${result.skipped.length} skipped`"
        color="neutral"
        variant="link"
        size="md"
        trailing-icon="i-lucide-chevron-down"
        class="group self-start px-0"
        :ui="{ trailingIcon: 'transition-transform group-data-[state=open]:rotate-180' }"
      />
      <template #content>
        <ul class="list-disc ps-5 text-sm">
          <li v-for="s in result.skipped" :key="s.title + s.reason">
            {{ s.title || 'Untitled' }}: <span class="text-muted">{{ s.reason }}</span>
          </li>
        </ul>
      </template>
    </UCollapsible>
  </form>
</template>
