<script setup lang="ts">
import type { DepictionTemplate } from '~/types/api';

const props = defineProps<{ smiles: string; template?: DepictionTemplate; fit?: boolean }>();
const emit = defineEmits<{ valid: [boolean] }>();
const svg = ref('');
const error = ref('');

watchEffect(async () => {
  error.value = '';
  try {
    svg.value = await renderStructure(props.smiles, props.template, 240, props.fit);
    emit('valid', true);
  } catch (e) {
    emit('valid', false);
    svg.value = '';
    error.value = e instanceof Error ? e.message : String(e);
  }
});
</script>

<template>
  <!-- Structures are drawn on white "paper" in both themes, so they read the same in dark mode. -->
  <div
    class="mx-auto flex aspect-square items-center justify-center bg-white"
    :class="fit ? 'w-full rounded-[inherit]' : 'w-full max-w-60 rounded-lg'"
    role="img"
    aria-label="Chemical structure"
  >
    <div v-if="svg" class="w-full [&_svg]:block [&_svg]:h-auto [&_svg]:w-full" v-html="svg" />
    <p v-else-if="error" class="p-4 text-sm text-neutral-600">{{ error }}</p>
    <p v-else class="p-4 text-sm text-neutral-600">Drawing…</p>
  </div>
</template>
