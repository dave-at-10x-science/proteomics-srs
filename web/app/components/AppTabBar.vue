<script setup lang="ts">
// App-style navigation along the bottom of the screen, within reach of a thumb.
const route = useRoute();

const tabs = [
  { label: 'Today', icon: 'i-lucide-house', to: '/', match: ['/review', '/study'] },
  { label: 'Browse', icon: 'i-lucide-layers', to: '/browse', match: [] },
  { label: 'Game', icon: 'i-lucide-puzzle', to: '/game', match: [] },
  { label: 'Library', icon: 'i-lucide-library-big', to: '/library', match: ['/sets', '/notes'] },
  { label: 'Settings', icon: 'i-lucide-settings', to: '/settings', match: [] },
];

const isActive = (t: (typeof tabs)[number]) =>
  route.path === t.to || t.match.some((p) => route.path === p || route.path.startsWith(`${p}/`));
</script>

<template>
  <nav
    aria-label="Main"
    class="fixed inset-x-0 bottom-0 z-10 border-t border-default bg-default/85 pb-[env(safe-area-inset-bottom)] backdrop-blur-lg"
  >
    <div class="mx-auto grid max-w-xl grid-cols-5 px-1">
      <ULink
        v-for="t in tabs"
        :key="t.to"
        :to="t.to"
        :active="isActive(t)"
        raw
        class="group flex flex-col items-center gap-1 pt-2 pb-1.5 text-[0.7rem] font-medium"
        active-class="text-primary"
        inactive-class="text-muted hover:text-default"
        :aria-current="isActive(t) ? 'page' : undefined"
      >
        <span
          class="flex h-7 w-12 items-center justify-center rounded-full transition-colors"
          :class="isActive(t) ? 'bg-primary/12' : 'group-hover:bg-elevated'"
        >
          <UIcon :name="t.icon" class="size-5" />
        </span>
        {{ t.label }}
      </ULink>
    </div>
  </nav>
</template>
