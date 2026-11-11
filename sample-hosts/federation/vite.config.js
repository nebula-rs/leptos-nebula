// Module Federation host. `remotes` is empty until the nebula `wc` builds exist (R-51/R-52);
// each slot in `src/slots.js` will name its remote and exposed module.
import { defineConfig } from 'vite'
import { nebulaDevMounts, nebulaDevMountsBuild } from '@leptos-nebula/dev-mounts/plugin'
import { federation } from '@module-federation/vite'

export default defineConfig({
  server: { port: 5301, strictPort: true },
  preview: { port: 5301, strictPort: true },
  build: { target: 'esnext' },
  plugins: [
    nebulaDevMounts(),
    nebulaDevMountsBuild(),
    federation({
      name: 'host_federation',
      remotes: {},
      shared: {},
      dts: false,
    }),
  ],
})
