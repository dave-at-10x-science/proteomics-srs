<script setup lang="ts">
import { marked } from 'marked';
import type { FieldDef, FieldValue, NoteType } from '~/types/api';

const props = defineProps<{
  def: FieldDef;
  fields: Record<string, FieldValue>;
  noteType: NoteType;
  label?: boolean;
  /** The big centered value of a review prompt. */
  large?: boolean;
  /** A short value in a row of three, which may wrap and hyphenate. */
  compact?: boolean;
  /** Emphasise this field, e.g. the answer. */
  highlight?: boolean;
}>();

const value = computed(() => props.fields[props.def.name]);
const structureSmiles = computed(() => {
  if (props.def.type === 'smiles') return String(value.value ?? '');
  if (props.def.type === 'image' && props.def.from) return String(props.fields[props.def.from] ?? '');
  return '';
});
const template = computed(() => {
  const id = props.def.depiction_template;
  return id ? props.noteType.depiction_templates?.[id] : undefined;
});
const html = computed(() => (props.def.type === 'markdown' ? (marked.parse(String(value.value ?? '')) as string) : ''));
</script>

<template>
  <div :class="{ 'border-s-3 border-primary ps-2.5': highlight }">
    <div v-if="label" class="mb-0.5 text-xs font-medium text-muted">{{ fieldLabel(def) }}</div>
    <StructureView v-if="structureSmiles" :smiles="structureSmiles" :template="template" />
    <img
      v-else-if="def.type === 'image' && value"
      :src="String(value)"
      :alt="fieldLabel(def)"
      class="mx-auto block max-h-80 max-w-full rounded-lg"
    />
    <!-- Markdown from notes: Tailwind's reset removes list and link styles, so they're restored here. -->
    <div
      v-else-if="def.type === 'markdown'"
      class="text-pretty [&_a]:text-primary [&_a]:underline [&_code]:font-mono [&_code]:text-[0.9em] [&_ol]:list-decimal [&_ol]:ps-5 [&_p+p]:mt-2 [&_ul]:list-disc [&_ul]:ps-5"
      :class="highlight ? 'text-highlighted' : 'text-default'"
      v-html="html"
    />
    <div
      v-else
      :class="[
        large
          ? 'text-center text-4xl font-bold tracking-tight text-highlighted'
          : compact
            ? 'text-base wrap-break-word hyphens-auto'
            : 'text-lg',
        highlight ? 'font-bold text-highlighted' : large ? '' : 'text-default',
      ]"
    >
      {{ value }}<span v-if="def.unit" class="ms-1 text-[0.8em] font-normal whitespace-nowrap text-muted">{{ def.unit }}</span>
    </div>
  </div>
</template>
