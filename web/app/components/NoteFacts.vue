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

// A short field with a long word ("Carbamidomethylation") takes two or three columns, so it is
// never broken mid-word on a phone.
function span(f: FieldDef) {
  const longest = Math.max(...String(props.fields[f.name]).split(/\s+/).map((w) => w.length));
  return longest > 20 ? 3 : longest > 13 ? 2 : 1;
}

// The widest span is the full row, so it can't add a column when the grid is two wide.
function spanStyle(f: FieldDef) {
  const n = span(f);
  return n === 3 ? { gridColumn: '1 / -1' } : n === 2 ? { gridColumn: 'span 2' } : undefined;
}

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
        :style="spanStyle(f)"
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
  /* Columns share the width by content, so a long name gets more room than a one-letter code. */
  grid-template-columns: repeat(3, minmax(min-content, auto));
  grid-auto-flow: dense;
  gap: 0.5rem 0.75rem;
}
@media (max-width: 340px) {
  .row {
    grid-template-columns: repeat(2, minmax(min-content, auto));
  }
}
.row :deep(.value) {
  font-size: 1.05rem;
  overflow-wrap: break-word;
  hyphens: auto;
}
.highlight {
  border-left: 3px solid var(--accent);
  padding-left: 0.5rem;
}
.highlight :deep(.value) {
  font-weight: 700;
}
</style>
