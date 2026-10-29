<script setup lang="ts">
/** One or more error messages, such as the server's validation errors, as an alert. Renders nothing when empty. */
const props = defineProps<{ errors: string | string[] }>();
const list = computed(() => (Array.isArray(props.errors) ? props.errors : [props.errors]).filter(Boolean));
</script>

<template>
  <UAlert
    v-if="list.length"
    color="error"
    variant="subtle"
    icon="i-lucide-circle-alert"
    :title="list.length === 1 ? list[0] : 'A few things need fixing'"
  >
    <template v-if="list.length > 1" #description>
      <ul class="list-disc ps-4">
        <li v-for="e in list" :key="e">{{ e }}</li>
      </ul>
    </template>
  </UAlert>
</template>
