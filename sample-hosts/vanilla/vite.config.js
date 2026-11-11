// Vite is only the dev server/bundler here: micro-frontend management is manual (`src/registry.js`).
import { defineConfig } from 'vite'
import { nebulaDevMounts, nebulaDevMountsBuild } from '@leptos-nebula/dev-mounts/plugin'

export default defineConfig({
  server: { port: 5302, strictPort: true },
  preview: { port: 5302, strictPort: true },
  build: { target: 'esnext' },
  plugins: [nebulaDevMounts(), nebulaDevMountsBuild()],
})
