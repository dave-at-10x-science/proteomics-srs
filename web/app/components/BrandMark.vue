<script setup lang="ts">
/**
 * The 10x Science mark (a polyhedron with an X), drawn from the logo's paths. The outline uses the
 * current text color; `accentClass` colors the X, which is purple in the logo.
 */
defineProps<{ accentClass?: string }>();
</script>

<template>
  <svg viewBox="0 0 80 77.2" fill="none" aria-hidden="true">
    <g stroke="currentColor" stroke-width="3" stroke-linejoin="round" transform="translate(-0.48)">
      <path d="M40.35,1.5L71,29.64,40.35,62.89,9.65,29.64Z" />
      <path d="M40.35,1.5L76.16,16.85,78.72,45,71,29.64Z" />
      <path d="M40.35,1.5L4.54,16.85,2,45,9.65,29.64Z" />
      <path d="M71,29.64L78.72,45,60.81,70.57,40.35,62.89Z" />
      <path d="M9.65,29.64L2,45,19.89,70.57l20.46-7.67Z" />
      <path d="M40.35,62.89l20.46,7.67L40.35,75.68,19.89,70.57Z" />
    </g>
    <g :class="accentClass" stroke="currentColor" stroke-width="3" fill="currentColor">
      <line x1="50.18" y1="20.79" x2="29.72" y2="41.24" />
      <line x1="50.18" y1="41.27" x2="29.72" y2="20.82" />
      <circle cx="50.16" cy="20.8" r="3.77" stroke="none" />
      <circle cx="29.72" cy="41.26" r="3.77" stroke="none" />
      <circle cx="50.18" cy="41.26" r="3.77" stroke="none" />
      <circle cx="29.72" cy="20.8" r="3.77" stroke="none" />
    </g>
  </svg>
</template>
