<script setup lang="ts">
import type { FieldDef, FieldValue, NoteType } from '~/types/api';

defineProps<{
  def: FieldDef;
  fields: Record<string, FieldValue>;
  noteType: NoteType;
}>();
const emit = defineEmits<{ close: [] }>();
</script>

<template>
  <UModal
    title="Diagram"
    :close="{ onClick: () => emit('close') }"
    :ui="{ content: 'max-w-3xl', body: 'p-2 sm:p-4' }"
  >
    <template #body>
      <FieldView
        :def="def"
        :fields="fields"
        :note-type="noteType"
        class="[&_img]:max-h-none [&_img]:w-full"
      />
    </template>
  </UModal>
</template>
