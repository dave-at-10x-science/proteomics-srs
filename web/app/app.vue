<template>
  <UApp :toaster="{ position: 'top-center', duration: 3000 }">
    <!-- Bottom padding leaves room for the tab bar. -->
    <div
      class="mx-auto flex min-h-dvh max-w-xl flex-col px-4 pt-[env(safe-area-inset-top)] pb-[calc(6rem+env(safe-area-inset-bottom))]"
    >
      <header class="flex items-center py-3">
        <NuxtLink to="/" class="flex items-center gap-2.5 font-semibold tracking-tight text-highlighted">
          <img src="/icon-192.png" alt="" class="size-8 rounded-lg shadow-sm" />
          Proteomics SRS
        </NuxtLink>
      </header>
      <main class="flex flex-1 flex-col">
        <NuxtPage />
      </main>
    </div>
    <AppTabBar />
  </UApp>
</template>
