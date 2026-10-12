// UnoCSS do próprio leptos-nebula: gera `style/dist/ui.css` a partir dos `.rs` do crate.
import { defineConfig, presetWind3 } from 'unocss'
import { presetUi } from './uno/preset.ts'

export default defineConfig({
  presets: [presetWind3(), presetUi()],
})
