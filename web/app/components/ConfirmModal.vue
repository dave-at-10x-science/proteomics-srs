<script setup lang="ts">
/** A yes/no question in a dialog. Open it with `useOverlay().create(ConfirmModal)`; resolves to true on confirm. */
withDefaults(
  defineProps<{
    title: string;
    description?: string;
    confirmLabel?: string;
    /** `error` for actions that destroy something. */
    color?: 'primary' | 'error';
  }>(),
  { confirmLabel: 'Confirm', color: 'primary', description: undefined },
);
const emit = defineEmits<{ close: [confirmed: boolean] }>();
</script>

<template>
  <UModal
    :title="title"
    :description="description"
    :close="{ onClick: () => emit('close', false) }"
    :ui="{ footer: 'flex flex-col-reverse gap-2' }"
  >
    <template #footer>
      <UButton label="Cancel" color="neutral" variant="outline" block @click="emit('close', false)" />
      <UButton :label="confirmLabel" :color="color" block @click="emit('close', true)" />
    </template>
  </UModal>
</template>
