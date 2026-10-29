<script setup lang="ts">
/** A row of equal-width numbers to pick one from, such as how many cards go in a round. */
defineProps<{ items: number[]; legend: string }>();
const model = defineModel<number>({ required: true });
</script>

<template>
  <URadioGroup
    v-model="model"
    :items="items"
    :legend="legend"
    variant="table"
    orientation="horizontal"
    indicator="hidden"
    :ui="{
      legend: 'mb-2 text-sm text-muted',
      fieldset: 'gap-0',
      item: 'flex-1 justify-center px-2 py-2.5 not-first-of-type:-ms-px',
      label: 'text-base font-semibold tabular-nums',
    }"
  />
</template>
