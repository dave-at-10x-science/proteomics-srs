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

async function upload(f: FieldDef, file: unknown) {
  if (!(file instanceof File)) return;
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
  <form class="flex flex-col gap-5" @submit.prevent="submit">
    <UFormField
      v-for="f in editable"
      :key="f.name"
      :label="fieldLabel(f)"
      :hint="f.optional ? 'Optional' : undefined"
      :error="badSmiles.includes(f) ? `That SMILES doesn't parse.` : undefined"
    >
      <UTextarea v-if="f.type === 'markdown'" v-model="values[f.name]" :rows="3" autoresize class="w-full" />

      <div v-else-if="f.type === 'image'" class="flex flex-col gap-2">
        <template v-if="text(f.name)">
          <img :src="text(f.name)" alt="" class="max-h-60 max-w-full self-start rounded-lg ring ring-default" />
          <UButton
            label="Remove image"
            icon="i-lucide-trash-2"
            color="error"
            variant="soft"
            size="md"
            class="self-start"
            @click="values[f.name] = ''"
          />
        </template>
        <UFileUpload
          v-else
          :model-value="null"
          accept="image/png,image/jpeg,image/webp,image/gif,image/svg+xml"
          icon="i-lucide-image-up"
          :label="uploading === f.name ? 'Uploading…' : 'Choose an image'"
          description="PNG, JPEG, WebP, GIF or SVG"
          :preview="false"
          :disabled="uploading === f.name"
          class="min-h-32 w-full"
          @update:model-value="upload(f, $event)"
        />
        <ErrorAlert :errors="uploadError" />
      </div>

      <UInput
        v-else
        v-model="values[f.name]"
        :type="f.type === 'number' ? 'number' : 'text'"
        :step="f.type === 'number' ? 'any' : undefined"
        :inputmode="f.type === 'number' ? 'decimal' : undefined"
        :autocapitalize="f.type === 'smiles' ? 'off' : undefined"
        :spellcheck="f.type === 'smiles' ? false : undefined"
        :ui="f.type === 'smiles' ? { base: 'font-mono' } : undefined"
        class="w-full"
      >
        <template v-if="f.unit" #trailing>
          <span class="text-sm text-muted">{{ f.unit }}</span>
        </template>
      </UInput>

      <!-- Live structure preview while typing SMILES. -->
      <StructureView
        v-if="f.type === 'smiles' && text(f.name)"
        :smiles="text(f.name)"
        :template="templateFor(f)"
        class="mt-3"
        @valid="smilesValid[f.name] = $event"
      />
    </UFormField>

    <USwitch v-model="draft" label="Draft" description="Not studied until published." />

    <ErrorAlert :errors="errors ?? []" />
    <p v-if="missing.length" class="text-sm text-muted">Needs: {{ missing.map((f) => fieldLabel(f)).join(', ') }}</p>

    <UButton type="submit" label="Save" icon="i-lucide-check" block :loading="saving" :disabled="!canSave" />
  </form>
</template>
