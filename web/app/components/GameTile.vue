<script setup lang="ts">
import type { FieldValue, NoteType } from '~/types/api';

const props = defineProps<{
  field: string;
  fields: Record<string, FieldValue>;
  noteType: NoteType;
  state: 'hidden' | 'shown' | 'matched';
}>();
defineEmits<{ flip: []; preview: [] }>();

const def = computed(() => props.noteType.fields.find((f) => f.name === props.field));
const smiles = computed(() => {
  const d = def.value;
  if (d?.type === 'smiles') return String(props.fields[d.name] ?? '');
  if (d?.type === 'image' && d.from) return String(props.fields[d.from] ?? '');
  return '';
});
const image = computed(() => {
  const d = def.value;
  if (d?.type !== 'image' || d.from) return '';
  const source = String(props.fields[d.name] ?? '').trim();
  return /^(?:https?:\/\/|data:image\/)/i.test(source)
    || /^(?:\/(?!\/)|\.{1,2}\/)[^\s]*$/.test(source)
    || /^(?:[^\s/?#:]+\/)*[^\s/?#:]+\.(?:avif|gif|jpe?g|png|svg|webp)(?:[?#][^\s]*)?$/i.test(source)
    ? source : '';
});
const template = computed(() => {
  const id = def.value?.depiction_template;
  return id ? props.noteType.depiction_templates?.[id] : undefined;
});
const text = computed(() => {
  const v = props.fields[props.field];
  return v == null ? '' : `${v}${def.value?.unit ? ` ${def.value.unit}` : ''}`;
});
const canPreview = computed(() => props.state !== 'hidden' && def.value?.detailed && !!(smiles.value || image.value));
</script>

<template>
  <div class="relative aspect-square min-w-0">
    <!-- A bespoke tile rather than a UButton: the card back and face are custom drawings. -->
    <button
      type="button"
      class="relative flex size-full items-center justify-center overflow-hidden rounded-lg transition duration-200 focus-visible:outline-3 focus-visible:outline-primary/40"
      :class="{
        'bg-linear-to-br from-primary-500 to-primary-700 shadow-sm active:scale-95': state === 'hidden',
        'bg-default ring-2 ring-primary shadow-md': state === 'shown',
        'bg-success/10 ring-2 ring-success/60 opacity-60': state === 'matched',
        'bg-white!': state !== 'hidden' && !!(smiles || image),
      }"
      :aria-label="state === 'hidden' ? 'Hidden card' : smiles ? 'Structure' : image ? 'Image' : text"
      :disabled="state !== 'hidden'"
      @click="$emit('flip')"
    >
      <!-- The card back is the 10x Science mark. -->
      <BrandMark v-if="state === 'hidden'" class="w-1/2 text-white/55" accent-class="text-white" />
      <!-- Structures are kept, so flipping back and forth doesn't redraw. -->
      <StructureView v-if="smiles" v-show="state !== 'hidden'" :smiles="smiles" :template="template" fit />
      <img v-else-if="image" v-show="state !== 'hidden'" :src="image" alt="" class="size-full object-contain p-1" />
      <span
        v-else-if="state !== 'hidden'"
        class="max-w-full p-1 leading-tight wrap-anywhere text-highlighted"
        :class="text.length > 30 ? 'text-[0.7rem]' : text.length > 20 ? 'text-xs' : text.length > 13 ? 'text-sm font-semibold' : 'text-base font-semibold'"
      >
        {{ text }}
      </span>
      <UIcon
        v-if="state === 'matched'"
        name="i-lucide-circle-check"
        class="absolute top-1 right-1 size-4 text-success"
      />
    </button>
    <UButton
      v-if="canPreview"
      icon="i-lucide-zoom-in"
      aria-label="Enlarge diagram"
      color="neutral"
      variant="outline"
      size="sm"
      class="absolute end-1 bottom-1 bg-default shadow-sm"
      @click="$emit('preview')"
    />
  </div>
</template>
