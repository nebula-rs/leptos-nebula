// Shortcuts do leptos-nebula (prefixo `lx-`). Cada componente usa só nomes daqui, escritos como
// literais nos `.rs` (o UnoCSS lê o fonte como texto).
//
// Regras (docs/guide/conventions.md):
// - nenhum nome começa com prefixo de variante do UnoCSS (todos começam com `lx-`);
// - estados mutuamente exclusivos são shortcuts COMPLETOS, e a base nunca define uma propriedade
//   que uma variante também define (a ordem no CSS não é a ordem das classes);
// - só tokens (`var(--lx-…)`): trocar de tema não muda nada aqui.

type Shortcuts = Record<string, string>

const v = (name: string) => `var(--lx-${name})`
/** Propriedade CSS arbitrária do UnoCSS: `[prop:valor]`. Espaços viram `_`. */
const p = (prop: string, value: string) => `[${prop}:${value.replaceAll(' ', '_')}]`
const sp = (n: string) => v(`space-${n}`)
const fs = (n: string) => p('font-size', v(`font-size-${n}`))
const color = (n: string) => p('color', v(`color-${n}`))
const bg = (n: string) => p('background', v(`color-${n}`))
const border = (side: string, token = 'color-border') =>
  p(`border-${side}`, `${v('border-width')} solid ${v(token)}`)

export const TONES = ['primary', 'secondary', 'positive', 'negative', 'info', 'warning', 'neutral'] as const

const tones: Shortcuts = Object.fromEntries(
  TONES.map((t) => [
    `lx-tone-${t}`,
    [
      p('--lx-tone', v(`color-${t}`)),
      p('--lx-tone-contrast', v(`color-on-${t}`)),
      p('--lx-tone-soft', v(`color-${t}-soft`)),
    ].join(' '),
  ]),
)

const ALIGN_JUSTIFY = {
  left: 'flex-start',
  center: 'center',
  right: 'flex-end',
  between: 'space-between',
  around: 'space-around',
  evenly: 'space-evenly',
  justify: 'stretch',
} as const

const cardActionsAlign: Shortcuts = Object.fromEntries(
  Object.entries(ALIGN_JUSTIFY).map(([k, j]) => [`lx-card-actions-align-${k}`, p('justify-content', j)]),
)
const tabsAlign: Shortcuts = Object.fromEntries(
  Object.entries(ALIGN_JUSTIFY).map(([k, j]) => [`lx-tabs-align-${k}`, p('justify-content', j)]),
)

const mtCells = (px: string, py: string) =>
  ['th', 'td'].map((c) => `[&_${c}]:px-[${px}] [&_${c}]:py-[${py}]`).join(' ')
const mtHoriz = ['th', 'td'].map((c) => `[&_${c}]:${border('bottom')}`).join(' ')
const mtVert = ['th+th', 'td+td'].map((c) => `[&_${c}]:${border('left')}`).join(' ')

export const shortcuts: Shortcuts = {
  ...tones,

  // ── Icon ──────────────────────────────────────────────────────────────────
  'lx-icon': 'inline-block shrink-0 align-middle',

  // ── Btn ───────────────────────────────────────────────────────────────────
  'lx-btn': [
    'relative inline-flex items-center justify-center box-border no-underline cursor-pointer select-none whitespace-nowrap',
    `gap-[${sp('sm')}] border border-solid`,
    fs('sm'),
    p('font-family', 'inherit'),
    p('font-weight', v('btn-weight')),
    p('text-transform', v('btn-transform')),
    p('letter-spacing', v('btn-tracking')),
    p('box-shadow', v('btn-shadow')),
    `transition-[filter,background-color] duration-[${v('duration-fast')}]`,
  ].join(' '),
  'lx-btn-size-normal': `h-[${v('control-height')}] px-[${sp('md')}] rounded-[${v('btn-radius')}]`,
  'lx-btn-size-dense': `h-[${v('control-height-sm')}] px-[${sp('sm')}] rounded-[${v('btn-radius')}]`,
  'lx-btn-size-round': `h-[${v('control-height')}] w-[${v('control-height')}] p-0 rounded-full`,
  'lx-btn-size-round-dense': `h-[${v('control-height-sm')}] w-[${v('control-height-sm')}] p-0 rounded-full`,
  'lx-btn-unelevated': [
    p('background', v('tone')),
    p('color', v('tone-contrast')),
    p('border-color', v('tone')),
    `hover:${p('filter', v('state-hover-filter'))}`,
    `active:${p('filter', v('state-active-filter'))}`,
  ].join(' '),
  'lx-btn-outline': [
    'bg-transparent',
    p('color', v('tone')),
    p('border-color', v('tone')),
    `hover:${p('background', v('tone-soft'))}`,
  ].join(' '),
  'lx-btn-flat': [
    'bg-transparent border-transparent',
    p('color', v('tone')),
    `hover:${p('background', v('tone-soft'))}`,
  ].join(' '),
  'lx-btn-disabled': `pointer-events-none cursor-not-allowed ${p('opacity', v('state-disabled-opacity'))}`,

  // ── Badge ─────────────────────────────────────────────────────────────────
  'lx-badge': [
    'inline-flex items-center box-border whitespace-nowrap border border-solid px-[0.375rem] py-[0.0625rem]',
    `rounded-[${v('badge-radius')}]`,
    fs('xs'),
    p('font-weight', v('font-weight-strong')),
  ].join(' '),
  'lx-badge-filled': `${p('background', v('tone'))} ${p('color', v('tone-contrast'))} ${p('border-color', v('tone'))}`,
  'lx-badge-outline': `bg-transparent ${p('color', v('tone'))} ${p('border-color', v('tone'))}`,
  'lx-badge-align-top': p('vertical-align', 'top'),
  'lx-badge-align-middle': p('vertical-align', 'middle'),
  'lx-badge-align-bottom': p('vertical-align', 'bottom'),

  // ── Banner ────────────────────────────────────────────────────────────────
  'lx-form': 'relative',

  'lx-banner': [
    'flex items-center box-border border border-solid',
    `gap-[${sp('md')}]`,
    p('background', v('tone-soft')),
    p('border-color', v('tone')),
    color('on-surface'),
    fs('sm'),
  ].join(' '),
  'lx-banner-normal': `px-[${sp('md')}] py-[${sp('md')}]`,
  'lx-banner-dense': `px-[${sp('sm')}] py-[${sp('sm')}]`,
  'lx-banner-rounded': `rounded-[${v('banner-radius')}]`,
  'lx-banner-square': 'rounded-none',
  'lx-banner-avatar': 'flex shrink-0 self-start',
  'lx-banner-content': 'flex-1 min-w-0',
  'lx-banner-actions': `flex shrink-0 items-center gap-[${sp('sm')}] self-end`,

  // ── Card ──────────────────────────────────────────────────────────────────
  'lx-card': [
    'block relative box-border overflow-hidden',
    p('background', v('card-bg')),
    color('on-surface'),
    p('backdrop-filter', v('card-backdrop')),
  ].join(' '),
  'lx-card-raised': p('box-shadow', v('card-shadow')),
  'lx-card-flat': 'shadow-none',
  'lx-card-bordered': p('border', v('card-border')),
  'lx-card-unbordered': 'border-none',
  'lx-card-round': p('border-radius', v('card-radius')),
  'lx-card-square': 'rounded-none',
  'lx-card-section-vertical': `block box-border p-[${sp('md')}]`,
  'lx-card-section-horizontal': `flex flex-row box-border p-[${sp('md')}] gap-[${sp('md')}]`,
  'lx-card-actions-row': `flex flex-row items-center box-border p-[${sp('sm')}] gap-[${sp('sm')}]`,
  'lx-card-actions-column': `flex flex-col items-stretch box-border p-[${sp('sm')}] gap-[${sp('sm')}]`,
  ...cardActionsAlign,

  // ── Separator ─────────────────────────────────────────────────────────────
  'lx-separator-h': `block w-full h-0 ${border('top')} shrink-0`,
  'lx-separator-v': `block self-stretch w-0 ${border('left')} shrink-0`,
  'lx-separator-spaced-h': `my-[${sp('md')}]`,
  'lx-separator-spaced-v': `mx-[${sp('md')}]`,
  'lx-separator-inset-h': `ml-[${sp('lg')}]`,
  'lx-separator-inset-v': `mt-[${sp('lg')}]`,

  // ── List / Item ───────────────────────────────────────────────────────────
  'lx-list': 'flex flex-col box-border m-0 p-0 list-none',
  'lx-list-bordered': `${p('border', `${v('border-width')} solid ${v('color-border')}`)} rounded-[${v('radius-md')}]`,
  'lx-list-separator': `[&>*+*]:${border('top')}`,
  'lx-list-padding': `py-[${sp('sm')}]`,
  'lx-item': [
    'relative flex flex-row items-center box-border no-underline',
    `gap-[${sp('md')}] rounded-[${v('item-radius')}]`,
    color('on-surface'),
    fs('sm'),
  ].join(' '),
  // `dense` reduz só a altura e o padding vertical (como o Quasar): o recuo horizontal é o
  // mesmo do item normal e do `ItemLabel header` (token único).
  'lx-item-size-normal': `min-h-[3rem] px-[${v('item-padding-x')}] py-[${sp('sm')}]`,
  'lx-item-size-dense': `min-h-[2.25rem] px-[${v('item-padding-x')}] py-[${sp('xs')}]`,
  'lx-item-clickable': `cursor-pointer ${p('transition', 'background-color 150ms')} hover:${p('background', v('item-hover-bg'))}`,
  'lx-item-active': `cursor-pointer ${p('background', v('item-active-bg'))} ${p('color', v('item-active-color'))}`,
  'lx-item-section-main': 'flex flex-col justify-center flex-1 min-w-0',
  'lx-item-section-avatar': 'flex flex-col justify-center flex-none min-w-[1.5rem] items-start',
  'lx-item-section-side': `flex flex-col justify-center flex-none items-end ${color('on-surface-muted')}`,
  'lx-item-label': 'block',
  // Espaço acima separa os grupos; o primeiro grupo não leva espaço extra.
  'lx-item-label-header': `block px-[${v('item-padding-x')}] pt-[${sp('lg')}] pb-[${sp('xs')}] first:pt-[${sp('sm')}] ${fs('xs')} ${color('on-surface-muted')} ${p('font-weight', v('font-weight-strong'))}`,
  'lx-item-label-caption': `block ${fs('xs')} ${color('on-surface-muted')}`,
  'lx-item-label-clamp': 'overflow-hidden [display:-webkit-box] [-webkit-box-orient:vertical]',

  // ── Layout / Header / Toolbar / Drawer / Page ─────────────────────────────
  'lx-layout': [
    'grid min-h-screen box-border',
    p('grid-template-columns', 'auto minmax(0,1fr) auto'),
    p('grid-template-rows', 'auto 1fr'),
    p('background', v('color-bg')),
    color('on-surface'),
    p('font-family', v('font-sans')),
  ].join(' '),
  'lx-header': [
    'sticky top-0 z-30 box-border',
    p('grid-column', '1/-1'),
    p('background', v('header-bg')),
    p('color', v('header-color')),
    p('backdrop-filter', v('header-backdrop')),
  ].join(' '),
  'lx-header-bordered': p('border-bottom', `${v('border-width')} solid ${v('header-border')}`),
  'lx-header-elevated': p('box-shadow', v('header-shadow')),
  'lx-toolbar': `flex items-center box-border w-full gap-[${sp('sm')}] px-[${sp('md')}] min-h-[${v('header-height')}]`,
  'lx-toolbar-title': `flex-1 min-w-0 overflow-hidden text-ellipsis whitespace-nowrap ${fs('lg')} ${p('font-weight', v('font-weight-strong'))}`,
  'lx-drawer-fixed-left': `sticky box-border shrink-0 overflow-y-auto ${p('grid-column', '1')} ${p('grid-row', '2')} top-[${v('header-height')}] h-[calc(100vh-${v('header-height')})] ${p('background', v('drawer-bg'))}`,
  'lx-drawer-fixed-right': `sticky box-border shrink-0 overflow-y-auto ${p('grid-column', '3')} ${p('grid-row', '2')} top-[${v('header-height')}] h-[calc(100vh-${v('header-height')})] ${p('background', v('drawer-bg'))}`,
  'lx-drawer-overlay-left': `fixed top-0 bottom-0 left-0 z-50 box-border overflow-y-auto max-w-[85vw] ${p('background', v('drawer-bg'))} ${p('box-shadow', v('drawer-shadow'))}`,
  'lx-drawer-overlay-right': `fixed top-0 bottom-0 right-0 z-50 box-border overflow-y-auto max-w-[85vw] ${p('background', v('drawer-bg'))} ${p('box-shadow', v('drawer-shadow'))}`,
  'lx-drawer-closed': 'hidden',
  'lx-drawer-border-r': border('right'),
  'lx-drawer-border-l': border('left'),
  'lx-drawer-backdrop': `fixed inset-0 z-40 ${p('background', v('color-scrim'))}`,
  'lx-page-container': `block min-w-0 box-border ${p('grid-column', '2')} ${p('grid-row', '2')}`,
  'lx-page-padded': `block box-border p-[${sp('lg')}]`,
  'lx-page-flush': 'block box-border',

  // ── Tabs ──────────────────────────────────────────────────────────────────
  'lx-tabs': `flex items-stretch overflow-x-auto box-border ${border('bottom')}`,
  ...tabsAlign,
  'lx-tab': [
    'relative inline-flex items-center justify-center whitespace-nowrap border-0 bg-transparent cursor-pointer box-border',
    `gap-[${sp('sm')}]`,
    fs('sm'),
    p('font-family', 'inherit'),
    p('font-weight', v('font-weight-strong')),
    `transition-colors duration-[${v('duration-fast')}]`,
  ].join(' '),
  'lx-tab-size-normal': `h-[${v('control-height')}] px-[${sp('md')}]`,
  'lx-tab-size-dense': `h-[${v('control-height-sm')}] px-[${sp('sm')}]`,
  'lx-tab-idle': `${color('on-surface-muted')} hover:${color('on-surface')}`,
  'lx-tab-active': `${p('color', v('tab-indicator'))} ${p('box-shadow', `inset 0 -2px 0 ${v('tab-indicator')}`)}`,
  'lx-tab-panels': 'block',
  'lx-tab-panel': 'block',

  // ── MarkupTable ───────────────────────────────────────────────────────────
  'lx-mt': [
    'block overflow-x-auto box-border',
    `rounded-[${v('radius-md')}]`,
    p('background', v('card-bg')),
    color('on-surface'),
    fs('sm'),
    '[&_table]:w-full [&_table]:border-collapse [&_table]:text-left',
    `[&_th]:${p('background', v('table-header-bg'))}`,
    `[&_th]:${p('color', v('table-header-color'))}`,
    `[&_th]:${p('font-weight', v('table-header-weight'))}`,
    `[&_th]:${p('text-transform', v('table-header-transform'))}`,
    `[&_tbody_tr:hover]:${p('background', v('table-row-hover'))}`,
  ].join(' '),
  'lx-mt-cells': mtCells(sp('md'), sp('sm')),
  'lx-mt-cells-dense': mtCells(sp('sm'), sp('xs')),
  'lx-mt-sep-horizontal': mtHoriz,
  'lx-mt-sep-vertical': mtVert,
  'lx-mt-sep-cell': `${mtHoriz} ${mtVert}`,
  'lx-mt-raised': p('box-shadow', v('shadow-sm')),
  'lx-mt-flat': 'shadow-none',
  'lx-mt-bordered': p('border', `${v('border-width')} solid ${v('color-border')}`),
  'lx-mt-unbordered': 'border-none',
  'lx-mt-wrap': '[&_td]:whitespace-normal [&_th]:whitespace-normal',
  'lx-mt-nowrap': '[&_td]:whitespace-nowrap [&_th]:whitespace-nowrap',
}
