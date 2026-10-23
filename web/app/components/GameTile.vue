<script setup lang="ts">
import type { FieldValue, NoteType } from '~/types/api';

const props = defineProps<{
  field: string;
  fields: Record<string, FieldValue>;
  noteType: NoteType;
  state: 'hidden' | 'shown' | 'matched';
}>();
defineEmits<{ flip: [] }>();

const def = computed(() => props.noteType.fields.find((f) => f.name === props.field));
const smiles = computed(() => {
  const d = def.value;
  if (d?.type === 'smiles') return String(props.fields[d.name] ?? '');
  if (d?.type === 'image' && d.from) return String(props.fields[d.from] ?? '');
  return '';
});
const template = computed(() => {
  const id = def.value?.depiction_template;
  return id ? props.noteType.depiction_templates?.[id] : undefined;
});
const text = computed(() => {
  const v = props.fields[props.field];
  return v == null ? '' : `${v}${def.value?.unit ? ` ${def.value.unit}` : ''}`;
});
</script>

<template>
  <button
    class="tile"
    :class="[state, { structure: !!smiles, long: text.length > 30 }]"
    :aria-label="state === 'hidden' ? 'Hidden card' : smiles ? 'Structure' : text"
    :disabled="state !== 'hidden'"
    @click="$emit('flip')"
  >
    <span v-if="state === 'hidden'" class="back" aria-hidden="true" />
    <!-- Structures are rendered once and kept, so flipping back and forth doesn't redraw. -->
    <StructureView v-if="smiles" v-show="state !== 'hidden'" :smiles="smiles" :template="template" fit />
    <span v-else-if="state !== 'hidden'" class="text">{{ text }}</span>
  </button>
</template>

<style scoped>
.tile {
  aspect-ratio: 1;
  padding: 0;
  min-height: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  position: relative;
  transition:
    transform 0.15s,
    opacity 0.3s;
}
.tile:disabled {
  opacity: 1;
  cursor: default;
}
.tile.hidden {
  background: var(--accent);
  border-color: var(--accent);
}
.tile.hidden:active {
  transform: scale(0.96);
}
.back {
  width: 40%;
  aspect-ratio: 1;
  border: 3px solid rgba(255, 255, 255, 0.55);
  border-radius: 50%;
}
.tile.structure.shown,
.tile.structure.matched {
  background: #fff;
}
.tile.shown {
  box-shadow: 0 0 0 3px var(--focus);
}
.tile.matched {
  border-color: var(--good);
  opacity: 0.55;
}
.text {
  padding: 0.3rem;
  font-weight: 600;
  font-size: 1rem;
  line-height: 1.2;
  overflow-wrap: anywhere;
}
.long .text {
  font-weight: 400;
  font-size: 0.7rem;
}
</style>
