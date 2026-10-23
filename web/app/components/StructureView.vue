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
  <!-- Structures are drawn on a light "paper" so they read the same in dark mode. -->
  <div class="structure" :class="{ fit }" role="img" :aria-label="`Chemical structure`">
    <div v-if="svg" class="svg" v-html="svg" />
    <p v-else-if="error" class="error">{{ error }}</p>
    <p v-else class="loading">Drawing…</p>
  </div>
</template>

<style scoped>
.structure {
  background: #fff;
  border-radius: var(--radius);
  display: flex;
  align-items: center;
  justify-content: center;
  aspect-ratio: 1;
  width: min(100%, 240px);
  margin: 0 auto;
  border: 1px solid var(--border);
}
.structure.fit {
  width: 100%;
  border: none;
  border-radius: inherit;
}
.svg {
  width: 100%;
}
.svg :deep(svg) {
  width: 100%;
  height: auto;
  display: block;
}
.loading,
.error {
  color: #666;
  font-size: 0.9rem;
  padding: 1rem;
}
</style>
