// Gera style/dist/ui.css (versionado). Ordem:
// reset/preflight do UnoCSS → primitives → tema scandi (+ media query p/ modo escuro) → base →
// utilitários/shortcuts usados em src/**/*.rs.
import { createGenerator } from 'unocss'
import { existsSync, readFileSync, writeFileSync, mkdirSync, readdirSync, statSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import config from '../uno.config.ts'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const read = (p) => readFileSync(join(root, p), 'utf8').trim()

function walk(dir) {
  return readdirSync(dir).flatMap((n) => {
    const f = join(dir, n)
    return statSync(f).isDirectory() ? walk(f) : f.endsWith('.rs') ? [f] : []
  })
}

/** Repete o bloco escuro (entre `dark:start`/`dark:end`) para quando `data-mode` estiver ausente. */
function autoDark(themeCss, theme) {
  const m = themeCss.match(/\/\* dark:start \*\/([\s\S]*?)\/\* dark:end \*\//)
  if (!m) throw new Error('marcadores dark:start/dark:end não encontrados')
  return `@media (prefers-color-scheme: dark) {\n  :root:not([data-mode]),\n  [data-theme="${theme}"]:not([data-mode]) {${m[1]}}\n}`
}

const uno = await createGenerator(config)
const code = walk(join(root, 'src')).map((f) => readFileSync(f, 'utf8')).join('\n')
const preflights = (await uno.generate('', { preflights: true })).css
const utilities = (await uno.generate(code, { preflights: false })).css

const scandi = read('style/tokens/themes/scandi.css')
const out = [
  '/* GERADO por scripts/build-css.mjs — não editar à mão. */',
  preflights,
  read('style/tokens/primitives.css'),
  scandi,
  autoDark(scandi, 'scandi'),
  read('style/base.css'),
  utilities,
].join('\n\n') + '\n'

const target = join(root, 'style/dist/ui.css')
if (process.argv.includes('--check')) {
  // css-fresh: falha se o arquivo versionado não é o que o build gera agora.
  const current = existsSync(target) ? readFileSync(target, 'utf8') : ''
  if (current !== out) {
    console.error('✗ style/dist/ui.css desatualizado — rode `moon run leptos-nebula:css`')
    process.exit(1)
  }
  console.log('✓ style/dist/ui.css em dia')
} else {
  mkdirSync(join(root, 'style/dist'), { recursive: true })
  writeFileSync(target, out)
  console.log(`✓ style/dist/ui.css (${out.length} bytes)`)
}
