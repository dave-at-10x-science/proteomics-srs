<script setup lang="ts">
import { marked } from 'marked';
import type { FieldDef, FieldValue, NoteType } from '~/types/api';

const props = defineProps<{
  def: FieldDef;
  fields: Record<string, FieldValue>;
  noteType: NoteType;
  label?: boolean;
  large?: boolean;
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
  <div class="field" :class="{ large }">
    <div v-if="label" class="label">{{ fieldLabel(def) }}</div>
    <StructureView v-if="structureSmiles" :smiles="structureSmiles" :template="template" />
    <img v-else-if="def.type === 'image' && value" :src="String(value)" :alt="fieldLabel(def)" class="image" />
    <div v-else-if="def.type === 'markdown'" class="markdown" v-html="html" />
    <div v-else class="value">
      {{ value }}<span v-if="def.unit" class="unit">{{ def.unit }}</span>
    </div>
  </div>
</template>

<style scoped>
.label {
  font-size: 0.8rem;
  color: var(--muted);
  margin-bottom: 0.15rem;
}
.value {
  font-size: 1.1rem;
}
.large .value {
  font-size: 2rem;
  font-weight: 600;
  text-align: center;
}
.unit {
  margin-left: 0.25em;
  color: var(--muted);
  font-size: 0.85em;
}
.image {
  display: block;
  max-width: 100%;
  max-height: 320px;
  margin: 0 auto;
  border-radius: var(--radius);
}
.markdown :deep(p) {
  margin: 0;
}
</style>
