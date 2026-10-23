<script setup lang="ts">
import type { FieldDef, FieldValue, NoteType } from '~/types/api';

const props = defineProps<{
  noteType: NoteType;
  initial?: Record<string, FieldValue>;
  initialStatus?: 'draft' | 'published';
  saving?: boolean;
  errors?: string[];
}>();
const emit = defineEmits<{ save: [fields: Record<string, FieldValue>, status: 'draft' | 'published'] }>();

// Inputs are edited as text (number inputs give numbers via v-model) and converted on save.
const values = reactive<Record<string, string | number>>({});
const text = (name: string) => String(values[name] ?? '').trim();
const draft = ref(props.initialStatus === 'draft');
const smilesValid = reactive<Record<string, boolean>>({});
const uploading = ref('');
const uploadError = ref('');

watchEffect(() => {
  for (const f of props.noteType.fields) {
    values[f.name] = props.initial?.[f.name] != null ? String(props.initial[f.name]) : '';
  }
});

const editable = computed(() => props.noteType.fields.filter((f) => !f.from));
// Structures drawn from another field are previewed under that field.
const derivedFrom = (f: FieldDef) => props.noteType.fields.find((d) => d.from === f.name);
const templateFor = (f: FieldDef) => {
  const id = derivedFrom(f)?.depiction_template;
  return id ? props.noteType.depiction_templates?.[id] : undefined;
};

const missing = computed(() => editable.value.filter((f) => !f.optional && !text(f.name)));
const badSmiles = computed(() =>
  editable.value.filter((f) => f.type === 'smiles' && text(f.name) && smilesValid[f.name] === false),
);
const canSave = computed(() => !props.saving && !missing.value.length && !badSmiles.value.length && !uploading.value);

async function upload(f: FieldDef, e: Event) {
  const file = (e.target as HTMLInputElement).files?.[0];
  if (!file) return;
  uploading.value = f.name;
  uploadError.value = '';
  try {
    values[f.name] = (await useApi().uploadImage(file)).url;
  } catch (err) {
    uploadError.value = apiErrors(err).join(' ');
  } finally {
    uploading.value = '';
  }
}

function submit() {
  if (!canSave.value) return;
  const fields: Record<string, FieldValue> = {};
  for (const f of editable.value) {
    const v = text(f.name);
    if (!v) continue;
    fields[f.name] = f.type === 'number' ? Number(v) : v;
  }
  emit('save', fields, draft.value ? 'draft' : 'published');
}
</script>

<template>
  <form class="stack" @submit.prevent="submit">
    <div v-for="f in editable" :key="f.name" class="field">
      <label :for="`f-${f.name}`">
        {{ fieldLabel(f) }}
        <span v-if="f.optional" class="muted">(optional)</span>
        <span v-if="f.unit" class="muted">({{ f.unit }})</span>
      </label>

      <textarea
        v-if="f.type === 'markdown'"
        :id="`f-${f.name}`"
        v-model="values[f.name]"
        rows="3"
      />
      <template v-else-if="f.type === 'image'">
        <img v-if="text(f.name)" :src="text(f.name)" alt="" class="preview-img" />
        <input :id="`f-${f.name}`" type="file" accept="image/png,image/jpeg,image/webp,image/gif,image/svg+xml" @change="upload(f, $event)" />
        <button v-if="values[f.name]" type="button" class="small" @click="values[f.name] = ''">Remove image</button>
        <p v-if="uploading === f.name" class="muted">Uploading…</p>
        <p v-if="uploadError" class="error">{{ uploadError }}</p>
      </template>
      <input
        v-else
        :id="`f-${f.name}`"
        v-model="values[f.name]"
        :type="f.type === 'number' ? 'number' : 'text'"
        :step="f.type === 'number' ? 'any' : undefined"
        :inputmode="f.type === 'number' ? 'decimal' : undefined"
        :autocapitalize="f.type === 'smiles' ? 'off' : undefined"
        :spellcheck="f.type === 'smiles' ? false : undefined"
        :class="{ mono: f.type === 'smiles' }"
      />

      <!-- Live structure preview while typing SMILES. -->
      <div v-if="f.type === 'smiles' && text(f.name)" class="preview">
        <StructureView
          :smiles="text(f.name)"
          :template="templateFor(f)"
          @valid="smilesValid[f.name] = $event"
        />
      </div>
    </div>

    <label class="row">
      <input v-model="draft" type="checkbox" />
      Draft (not studied until published)
    </label>

    <ul v-if="errors?.length" class="error">
      <li v-for="e in errors" :key="e">{{ e }}</li>
    </ul>
    <p v-if="missing.length" class="muted small">Needs: {{ missing.map((f) => fieldLabel(f)).join(', ') }}</p>
    <p v-if="badSmiles.length" class="error small">That SMILES doesn't parse.</p>

    <button class="primary" :disabled="!canSave">{{ saving ? 'Saving…' : 'Save' }}</button>
  </form>
</template>

<style scoped>
.field {
  display: grid;
  gap: 0.35rem;
}
label {
  font-weight: 600;
  font-size: 0.95rem;
}
label .muted {
  font-weight: 400;
}
textarea {
  width: 100%;
  padding: 0.75rem;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface);
  color: var(--text);
  font: inherit;
  font-size: 1rem;
  resize: vertical;
}
.mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 1rem;
}
.preview {
  margin-top: 0.25rem;
}
.preview-img {
  max-width: 100%;
  max-height: 240px;
  border-radius: var(--radius);
}
.row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-weight: 400;
}
.row input {
  width: 1.2rem;
  height: 1.2rem;
}
.small {
  font-size: 0.85rem;
}
button.small {
  min-height: 36px;
  padding: 0.3rem 0.75rem;
  justify-self: start;
}
.error {
  color: var(--again);
}
ul.error {
  margin: 0;
  padding-left: 1.2rem;
}
</style>
