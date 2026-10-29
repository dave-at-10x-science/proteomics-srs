<template>
  <UApp :toaster="{ position: 'top-center', duration: 3000 }">
    <!-- Bottom padding leaves room for the tab bar. -->
    <div
      class="mx-auto flex min-h-dvh max-w-xl flex-col px-4 pt-[env(safe-area-inset-top)] pb-[calc(6rem+env(safe-area-inset-bottom))]"
    >
      <header class="flex items-center py-3.5">
        <NuxtLink to="/" class="flex min-w-0 items-center gap-3">
          <UColorModeImage
            light="/10x-science-logo-light.svg"
            dark="/10x-science-logo.svg"
            alt="10x Science"
            class="h-5.5 w-auto shrink-0"
          />
          <USeparator orientation="vertical" class="h-5" />
          <span class="truncate font-semibold tracking-tight text-highlighted">Proteomic Pull-ups</span>
        </NuxtLink>
      </header>
      <main class="flex flex-1 flex-col">
        <NuxtPage />
      </main>
    </div>
    <AppTabBar />
  </UApp>
</template>
