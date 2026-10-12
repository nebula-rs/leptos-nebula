// Preset UnoCSS exportado pelo leptos-nebula: tema → tokens, shortcuts e ícones lucide.
// O app que usa o crate adiciona `presetUi()` ao próprio `uno.config.ts` para usar os mesmos
// shortcuts (`lx-btn`…) e cores (`bg-lx-primary`…) no markup dele. O CSS dos componentes do
// crate já vem pronto em `style/dist/ui.css` (ADR-0005).
import { presetIcons, type Preset } from 'unocss'
import { shortcuts, TONES } from './shortcuts.ts'

const v = (name: string) => `var(--lx-${name})`

const colors: Record<string, string> = {
  'lx-bg': v('color-bg'),
  'lx-surface': v('color-surface'),
  'lx-surface-muted': v('color-surface-muted'),
  'lx-surface-raised': v('color-surface-raised'),
  'lx-on-surface': v('color-on-surface'),
  'lx-on-surface-muted': v('color-on-surface-muted'),
  'lx-border': v('color-border'),
  'lx-border-strong': v('color-border-strong'),
  'lx-focus': v('color-focus'),
}
for (const t of TONES) {
  colors[`lx-${t}`] = v(`color-${t}`)
  colors[`lx-on-${t}`] = v(`color-on-${t}`)
  colors[`lx-${t}-soft`] = v(`color-${t}-soft`)
}

export function presetUi(): Preset {
  return {
    name: 'leptos-nebula',
    theme: { colors },
    shortcuts,
    // Ícones Iconify em CSS puro: i-lucide-<nome>. A coleção é carregada explicitamente
    // (com pnpm, o carregamento automático não acha o pacote).
    presets: [
      presetIcons({
        scale: 1.1,
        extraProperties: { 'display': 'inline-block', 'vertical-align': '-0.15em' },
        collections: {
          lucide: () =>
            import('@iconify-json/lucide/icons.json', { with: { type: 'json' } }).then((i) => i.default as any),
        },
      }),
    ],
  }
}
