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
// never broken mid-word on a phone. The widest span is the full row, so it can't add a column when
// the grid is two wide.
function span(f: FieldDef) {
  const longest = Math.max(...String(props.fields[f.name]).split(/\s+/).map((w) => w.length));
  return longest > 20 ? 'col-span-full' : longest > 13 ? 'col-span-2' : '';
}

const visuals = computed(() => shown.value.filter(isVisual));
const short = computed(() => shown.value.filter(isShort));
const long = computed(() => shown.value.filter((f) => !isVisual(f) && !isShort(f)));
</script>

<template>
  <div class="flex flex-col gap-4">
    <template v-if="visualsFirst">
      <FieldView v-for="f in visuals" :key="f.name" :def="f" :fields="fields" :note-type="noteType" />
    </template>
    <!-- Columns share the width by content, so a long name gets more room than a one-letter code. -->
    <div
      v-if="short.length"
      class="grid grid-flow-row-dense grid-cols-[repeat(3,minmax(min-content,auto))] gap-x-3 gap-y-3 max-[340px]:grid-cols-[repeat(2,minmax(min-content,auto))]"
    >
      <FieldView
        v-for="f in short"
        :key="f.name"
        :def="f"
        :fields="fields"
        :note-type="noteType"
        :highlight="highlight?.includes(f.name)"
        :class="span(f)"
        label
        compact
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
      :highlight="highlight?.includes(f.name)"
      label
    />
  </div>
</template>
