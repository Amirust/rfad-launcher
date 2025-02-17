export default defineNuxtConfig({
  compatibilityDate: '2024-11-01',
  devtools: { enabled: true },

  postcss: {
    plugins: {
      tailwindcss: {},
      autoprefixer: {}
    }
  },

  ssr: false,
  devServer: { host: process.env.TAURI_DEV_HOST || 'localhost' },
  css: [ '~/assets/css/global.scss' ],
  vite: {
    plugins: [],
    clearScreen: false,
    // https://v2.tauri.app/reference/environment-variables/
    envPrefix: [ 'VITE_', 'TAURI_' ],
    server: {
      strictPort: true
    }
  }
})
