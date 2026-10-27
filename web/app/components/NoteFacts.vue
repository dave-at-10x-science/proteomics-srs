<script setup lang="ts">
import type { FieldDef, FieldValue, NoteType } from '~/types/api';

/**
 * Everything about a note in a compact layout: short text and number fields three to a row
 * (name, three-letter and one-letter code fit across), structures and images full width, and
 * longer text below.
 */
const props = defineProps<{
  noteType: NoteType;
  fields: Record<string, FieldValue>;
  /** Fields to leave out, e.g. a structure already shown as the prompt. */
  exclude?: string[];
  /** Fields to emphasise, e.g. the answer. */
  highlight?: string[];
  /** Put structures and images above the facts instead of below. */
  visualsFirst?: boolean;
}>();

const LONG_TEXT = 40;

const present = (f: FieldDef) => props.fields[f.from ?? f.name] != null && props.fields[f.from ?? f.name] !== '';
const shown = computed(() =>
  // Raw SMILES strings are left out; the structure drawn from them is shown instead.
  props.noteType.fields.filter((f) => !props.exclude?.includes(f.name) && f.type !== 'smiles' && present(f)),
);
const isVisual = (f: FieldDef) => f.type === 'image';
const isShort = (f: FieldDef) =>
  (f.type === 'text' || f.type === 'number') && String(props.fields[f.name]).length <= LONG_TEXT;

const visuals = computed(() => shown.value.filter(isVisual));
const short = computed(() => shown.value.filter(isShort));
const long = computed(() => shown.value.filter((f) => !isVisual(f) && !isShort(f)));
</script>

<template>
  <div class="facts">
    <template v-if="visualsFirst">
      <FieldView v-for="f in visuals" :key="f.name" :def="f" :fields="fields" :note-type="noteType" />
    </template>
    <div v-if="short.length" class="row">
      <FieldView
        v-for="f in short"
        :key="f.name"
        :def="f"
        :fields="fields"
        :note-type="noteType"
        :class="{ highlight: highlight?.includes(f.name) }"
        label
      />
    </div>
    <template v-if="!visualsFirst">
      <FieldView v-for="f in visuals" :key="f.name" :def="f" :fields="fields" :note-type="noteType" label />
    </template>
    <FieldView
      v-for="f in long"
      :key="f.name"
      :def="f"
      :fields="fields"
      :note-type="noteType"
      :class="{ highlight: highlight?.includes(f.name) }"
      label
    />
  </div>
</template>

<style scoped>
.facts {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}
.row {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.5rem 0.75rem;
}
.row :deep(.value) {
  font-size: 1.05rem;
  overflow-wrap: anywhere;
}
.highlight {
  border-left: 3px solid var(--accent);
  padding-left: 0.5rem;
}
.highlight :deep(.value) {
  font-weight: 700;
}
</style>
