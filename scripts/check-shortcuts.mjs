// Garante que todo shortcut do uno.config.ts gera CSS.
// O UnoCSS descarta um shortcut em silêncio quando o nome colide com um prefixo de
// variante (ex.: `empty-state` = variante `empty` + `state`) ou quando nenhum utilitário casa.
import { createGenerator } from 'unocss'
import config from '../uno.config.ts'
import { shortcuts } from '../uno/shortcuts.ts'

const uno = await createGenerator(config)
const failures = []
for (const [name, body] of Object.entries(shortcuts)) {
  if (!body.trim()) continue // shortcut vazio de propósito (ex.: estado sem estilo)
  const { css } = await uno.generate(name, { preflights: false })
  if (css.trim()) continue
  const parts = body.split(/\s+(?![^(]*\))/)
  const unmatched = []
  for (const part of parts) {
    const { css: partCss } = await uno.generate(part, { preflights: false })
    if (!partCss.trim()) unmatched.push(part)
  }
  failures.push({ name, unmatched })
}

// Item (normal e dense) e rótulo de grupo compartilham o mesmo token de padding horizontal:
// se divergirem, o texto dos itens sai desalinhado do rótulo do grupo.
for (const name of ['lx-item-size-normal', 'lx-item-size-dense', 'lx-item-label-header']) {
  if (!shortcuts[name]?.includes('px-[var(--lx-item-padding-x)]')) {
    failures.push({ name, unmatched: ['px-[var(--lx-item-padding-x)] (padding horizontal compartilhado)'] })
  }
}

if (failures.length) {
  for (const f of failures) {
    console.error(`✗ ${f.name}: ${f.unmatched.length ? `utilitários sem match: ${f.unmatched.join(', ')}` : 'nome colide com uma variante do UnoCSS (renomeie)'}`)
  }
  process.exit(1)
}
console.log(`✓ ${Object.keys(shortcuts).length} shortcuts geram CSS`)
