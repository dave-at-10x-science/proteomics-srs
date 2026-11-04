<script setup lang="ts">
import type { SubscriptionProgress } from '~/types/api';

defineProps<{
  setName: string;
  studiedCount: number;
}>();
const emit = defineEmits<{ close: [progress?: SubscriptionProgress] }>();
</script>

<template>
  <UModal
    title="Resume studying this set?"
    :description="`You have previous progress on ${studiedCount} ${studiedCount === 1 ? 'concept' : 'concepts'} in ${setName}.`"
    :close="{ onClick: () => emit('close') }"
    :ui="{ body: 'flex flex-col gap-3', footer: 'flex flex-col gap-2' }"
  >
    <template #body>
      <p><strong>Resume reviews</strong> keeps your existing progress and review schedule.</p>
      <p>
        <strong>Start over</strong> clears study and review progress for all concepts in this set.
        You'll study them again before reviewing.
      </p>
    </template>
    <template #footer>
      <UButton label="Resume reviews" block @click="emit('close', 'resume')" />
      <UButton label="Start over" color="error" variant="outline" block @click="emit('close', 'restart')" />
      <UButton label="Cancel" color="neutral" variant="ghost" block @click="emit('close')" />
    </template>
  </UModal>
</template>
