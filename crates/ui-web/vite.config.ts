import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import checker from 'vite-plugin-checker'

export default defineConfig({
  plugins: [
    react(),
    checker({
      // keep type checking in the overlay
      typescript: true,
      // TEMP: disable ESLint overlay to avoid the v9 options error
      // eslint: { lintCommand: 'eslint "./src/**/*.{ts,tsx}"' }, // <- leave this out for now
    }),
  ],
})
