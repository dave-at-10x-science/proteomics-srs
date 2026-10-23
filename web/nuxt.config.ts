// Single-page app served by srs-server. `npm run build` writes static files to .output/public,
// which the server picks up (SRS_WEB_DIR). In development, /api is proxied to the server.
export default defineNuxtConfig({
  compatibilityDate: '2026-10-01',
  ssr: false,
  devtools: { enabled: false },
  css: ['~/assets/main.css'],
  modules: ['@vite-pwa/nuxt'],
  app: {
    head: {
      title: 'Proteomics SRS',
      meta: [
        { name: 'viewport', content: 'width=device-width, initial-scale=1, viewport-fit=cover' },
        { name: 'theme-color', content: '#1f5f8b' },
        { name: 'apple-mobile-web-app-capable', content: 'yes' },
      ],
      link: [{ rel: 'apple-touch-icon', href: '/icon-192.png' }],
    },
  },
  nitro: {
    devProxy: {
      '/api': { target: process.env.SRS_API ?? 'http://localhost:8080/api', changeOrigin: true },
    },
  },
  pwa: {
    registerType: 'autoUpdate',
    manifest: {
      name: 'Proteomics SRS',
      short_name: 'SRS',
      description: 'Daily spaced repetition for proteomics and mass spectrometry',
      theme_color: '#1f5f8b',
      background_color: '#f7f8fa',
      display: 'standalone',
      start_url: '/',
      icons: [
        { src: '/icon-192.png', sizes: '192x192', type: 'image/png' },
        { src: '/icon-512.png', sizes: '512x512', type: 'image/png' },
        { src: '/icon-512.png', sizes: '512x512', type: 'image/png', purpose: 'maskable' },
      ],
    },
    workbox: {
      navigateFallback: '/',
      // The app shell and RDKit are cached; the API always goes to the network.
      globPatterns: ['**/*.{js,css,html,png,svg,wasm}'],
      maximumFileSizeToCacheInBytes: 12 * 1024 * 1024,
      navigateFallbackDenylist: [/^\/api\//],
    },
  },
});
