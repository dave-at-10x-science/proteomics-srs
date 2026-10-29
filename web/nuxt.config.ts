// Single-page app served by srs-server. `npm run build` writes static files to .output/public,
// which the server picks up (SRS_WEB_DIR). In development, /api is proxied to the server.
export default defineNuxtConfig({
  compatibilityDate: '2026-10-01',
  ssr: false,
  devtools: { enabled: false },
  css: ['~/assets/main.css'],
  modules: ['@nuxt/ui', '@vite-pwa/nuxt'],
  ui: {
    // System fonts: nothing to download, and they match the phone.
    fonts: false,
    theme: {
      // Large controls by default, so everything is comfortable to tap.
      defaultVariants: { size: 'xl' },
    },
    // Only generate CSS for the components the app uses.
    experimental: { componentDetection: true },
  },
  icon: {
    // Icons are bundled at build time and never fetched, so the installed app works offline. The
    // scan only finds literal `i-lucide-*` names in .vue files.
    provider: 'none',
    clientBundle: { scan: true },
  },
  app: {
    head: {
      title: 'Proteomic Pull-ups',
      // Lets the browser hyphenate long words in narrow columns.
      htmlAttrs: { lang: 'en' },
      meta: [
        { name: 'viewport', content: 'width=device-width, initial-scale=1, viewport-fit=cover' },
        // The browser and status bars match the page background (gray-100 and gray-950).
        { name: 'theme-color', content: '#f3f4f6', media: '(prefers-color-scheme: light)' },
        { name: 'theme-color', content: '#030712', media: '(prefers-color-scheme: dark)' },
        { name: 'apple-mobile-web-app-capable', content: 'yes' },
        // The home screen label; the full name gets cut off there.
        { name: 'apple-mobile-web-app-title', content: 'Pull-ups' },
      ],
      link: [
        { rel: 'icon', type: 'image/png', href: '/icon-192.png' },
        // iOS draws its own rounded corners and shows transparency as black, so this one is square.
        { rel: 'apple-touch-icon', href: '/apple-touch-icon.png' },
      ],
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
      name: 'Proteomic Pull-ups',
      short_name: 'Pull-ups',
      description: 'Daily spaced repetition for proteomics and mass spectrometry',
      theme_color: '#f3f4f6',
      background_color: '#f3f4f6',
      display: 'standalone',
      start_url: '/',
      icons: [
        { src: '/icon-192.png', sizes: '192x192', type: 'image/png' },
        { src: '/icon-512.png', sizes: '512x512', type: 'image/png' },
        // Full-bleed, with the mark inside the safe zone that Android's shaped masks keep.
        { src: '/icon-maskable-512.png', sizes: '512x512', type: 'image/png', purpose: 'maskable' },
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
