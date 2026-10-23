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
  <form class="stack" @submit.prevent="submit">
    <label class="field">
      <span>What should the cards cover?</span>
      <textarea
        v-model="source"
        rows="4"
        placeholder="A topic (“proteases used in sample prep”), a list of terms, or text pasted from a paper"
      />
    </label>
    <div class="inline">
      <label class="field count">
        <span>How many</span>
        <input v-model.number="count" type="number" min="1" max="30" />
      </label>
      <label v-if="empty" class="field grow">
        <span>Card type</span>
        <select v-model="noteTypeId">
          <option v-for="t in noteTypes" :key="t.id" :value="t.id">{{ t.id }}</option>
        </select>
      </label>
    </div>
    <button class="primary" :disabled="!canDraft">{{ busy ? 'Drafting… this can take a minute' : 'Draft' }}</button>
    <ul v-if="errors.length" class="error">
      <li v-for="e in errors" :key="e">{{ e }}</li>
    </ul>
    <div v-if="result" class="result">
      <p>
        <strong>{{ result.created.length }} {{ result.created.length === 1 ? 'draft' : 'drafts' }} added.</strong>
        <span class="muted"> Check each one before publishing; drafts are never studied.</span>
      </p>
      <details v-if="result.skipped.length">
        <summary class="muted">{{ result.skipped.length }} skipped</summary>
        <ul class="skipped">
          <li v-for="s in result.skipped" :key="s.title + s.reason">
            {{ s.title || 'Untitled' }}: <span class="muted">{{ s.reason }}</span>
          </li>
        </ul>
      </details>
    </div>
  </form>
</template>

<style scoped>
.field {
  display: grid;
  gap: 0.35rem;
  font-weight: 600;
  font-size: 0.95rem;
}
textarea,
select {
  width: 100%;
  padding: 0.75rem;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface);
  color: var(--text);
  font: inherit;
  font-size: 1rem;
  font-weight: 400;
}
textarea {
  resize: vertical;
}
select {
  min-height: 48px;
}
.inline {
  display: flex;
  gap: 0.75rem;
}
.count {
  width: 6.5rem;
}
.grow {
  flex: 1;
}
.result p {
  margin: 0;
}
.skipped {
  margin: 0.5rem 0 0;
  padding-left: 1.2rem;
  font-size: 0.9rem;
}
.error {
  color: var(--again);
  margin: 0;
  padding-left: 1.2rem;
}
</style>
