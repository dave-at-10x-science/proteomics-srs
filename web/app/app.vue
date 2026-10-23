<template>
  <div class="shell">
    <header>
      <NuxtLink to="/" class="title">Proteomics SRS</NuxtLink>
      <nav>
        <NuxtLink to="/settings" aria-label="Settings">Settings</NuxtLink>
      </nav>
    </header>
    <main>
      <NuxtPage />
    </main>
  </div>
</template>

<style scoped>
.shell {
  max-width: 560px;
  margin: 0 auto;
  padding: 0 16px calc(16px + env(safe-area-inset-bottom));
  min-height: 100dvh;
  display: flex;
  flex-direction: column;
}
header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: calc(12px + env(safe-area-inset-top)) 0 12px;
}
.title {
  font-weight: 700;
  color: var(--text);
  text-decoration: none;
}
nav a {
  color: var(--muted);
  text-decoration: none;
  font-size: 0.95rem;
}
main {
  flex: 1;
  display: flex;
  flex-direction: column;
}
</style>
