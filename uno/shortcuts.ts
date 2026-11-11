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

// ── Dialog ────────────────────────────────────────────────────────────────
const DIALOG_POSITIONS = ['standard', 'top', 'bottom', 'left', 'right'] as const
const dialogAlign = {
  standard: 'items-center justify-center',
  top: 'items-start justify-center',
  bottom: 'items-end justify-center',
  left: 'items-center justify-start',
  right: 'items-center justify-end',
} as const
const dpad = `[${v('dialog-padding')}]`
// padding da área do painel: o lado encostado na borda não tem folga (em tela estreita,
// top/bottom também não têm folga lateral)
const dialogPad = {
  standard: `p-${dpad}`,
  top: `pt-0 pb-0 px-${dpad} max-sm:px-0`,
  bottom: `pt-0 pb-0 px-${dpad} max-sm:px-0`,
  left: `px-0 py-${dpad}`,
  right: `px-0 py-${dpad}`,
} as const
const dr = v('dialog-radius')
const dialogRadius = {
  standard: dr,
  top: `0 0 ${dr} ${dr}`,
  bottom: `${dr} ${dr} 0 0`,
  left: `0 ${dr} ${dr} 0`,
  right: `${dr} 0 0 ${dr}`,
} as const
const dist = v('dialog-slide-distance')
const dialogFrom = {
  standard: `scale(${v('dialog-scale-from')})`,
  top: `translateY(calc(-1 * ${dist}))`,
  bottom: `translateY(${dist})`,
  left: `translateX(calc(-1 * ${dist}))`,
  right: `translateX(${dist})`,
} as const

const dialogInner: Shortcuts = {
  'lx-dialog-inner-max': 'flex box-border w-full h-full p-0 pointer-events-none',
  ...Object.fromEntries(
    DIALOG_POSITIONS.map((pos) => [
      `lx-dialog-inner-${pos}`,
      `flex box-border w-full h-full pointer-events-none ${dialogAlign[pos]} ${dialogPad[pos]}`,
    ]),
  ),
}
const dialogBody: Shortcuts = Object.fromEntries(
  DIALOG_POSITIONS.map((pos) => [
    `lx-dialog-body-${pos}`,
    [p('border-radius', dialogRadius[pos]), pos === 'top' || pos === 'bottom' ? 'max-sm:w-full' : ''].join(' ').trim(),
  ]),
)
// Notify: uma pilha por posição. No topo o mais novo fica em cima (coluna invertida).
const NOTIFY_STACKS: Record<string, string> = {
  'top-left': 'top-0 left-0 items-start flex-col-reverse',
  top: 'top-0 inset-x-0 items-center flex-col-reverse',
  'top-right': 'top-0 right-0 items-end flex-col-reverse',
  left: 'top-1/2 left-0 -translate-y-1/2 items-start flex-col',
  center: 'top-1/2 inset-x-0 -translate-y-1/2 items-center flex-col',
  right: 'top-1/2 right-0 -translate-y-1/2 items-end flex-col',
  'bottom-left': 'bottom-0 left-0 items-start flex-col',
  bottom: 'bottom-0 inset-x-0 items-center flex-col',
  'bottom-right': 'bottom-0 right-0 items-end flex-col',
}
const notifyStacks: Shortcuts = Object.fromEntries(
  Object.entries(NOTIFY_STACKS).map(([pos, utils]) => [`lx-notify-stack-${pos}`, utils]),
)

const dialogEnter: Shortcuts = Object.fromEntries(
  DIALOG_POSITIONS.map((pos) => [
    `lx-dialog-enter-${pos}`,
    // reduced-motion: só o fade, sem deslocamento
    `${p('opacity', '0')} ${p('transform', dialogFrom[pos])} motion-reduce:${p('transform', 'none')}`,
  ]),
)

const mtCells = (px: string, py: string) =>
  ['th', 'td'].map((c) => `[&_${c}]:px-[${px}] [&_${c}]:py-[${py}]`).join(' ')
const mtHoriz = ['th', 'td'].map((c) => `[&_${c}]:${border('bottom')}`).join(' ')
const mtVert = ['th+th', 'td+td'].map((c) => `[&_${c}]:${border('left')}`).join(' ')

// ── Field ─────────────────────────────────────────────────────────────────
// Design × foco são shortcuts completos na caixa do controle; o label tem dois estados
// exclusivos (repouso × flutuando) cujas posições vêm de tokens do tema (ADR-0010).
const fieldRing = (token: string) => p('box-shadow', `0 0 0 ${v('field-focus-ring-width')} ${token}`)
const fieldUnderline = (token: string) => p('box-shadow', `0 1px 0 0 ${token}`)
type FieldState = 'idle' | 'focused' | 'error'
const fieldBoxVisual = (design: string, state: FieldState): string => {
  const w = v('border-width')
  // O erro usa a cor `negative` (como o foco usa o tom do campo) e vence o foco.
  const focused = state !== 'idle'
  const tone = state === 'error' ? v('color-negative') : v('tone')
  const idleBorder = v('field-border')
  switch (design) {
    case 'outlined':
      return [
        p('background', v('field-bg')),
        p('border-radius', v('field-radius')),
        p('border-style', 'solid'),
        p('border-width', w),
        p('border-color', focused ? tone : idleBorder),
        focused ? fieldRing(tone) : '',
      ].join(' ')
    case 'filled':
      return [
        p('background', v('field-filled-bg')),
        p('border-radius', v('field-filled-radius')),
        p('border-style', 'solid'),
        p('border-width', w),
        p('border-color', `transparent transparent ${focused ? tone : idleBorder} transparent`),
        focused ? fieldUnderline(tone) : '',
      ].join(' ')
    default:
      return [
        'bg-transparent',
        p('border-radius', v('field-radius')),
        p('border-style', 'solid'),
        p('border-width', w),
        'border-transparent',
        focused ? fieldUnderline(tone) : '',
      ].join(' ')
  }
}
const fieldBoxes: Shortcuts = Object.fromEntries(
  ['outlined', 'filled', 'borderless'].flatMap((d) => [
    [`lx-field-${d}-idle`, fieldBoxVisual(d, 'idle')],
    [`lx-field-${d}-focused`, fieldBoxVisual(d, 'focused')],
    [`lx-field-${d}-error`, fieldBoxVisual(d, 'error')],
  ]),
)
const fieldLabel = (transform: string, bgToken: string | null) =>
  [
    'absolute left-0 top-0 z-[1] box-border max-w-full overflow-hidden text-ellipsis whitespace-nowrap',
    p('transform-origin', 'left top'),
    p('transform', v(transform)),
    bgToken ? p('background', v(bgToken)) : 'bg-transparent',
    p('pointer-events', v('field-label-pointer-events')),
    p('color', v('field-label-color')),
    p('font-size', v('field-label-font-size')),
    p('font-weight', v('field-label-weight')),
    p('line-height', v('field-label-line-height')),
    p('transition', v('field-label-transition')),
  ].join(' ')

const field: Shortcuts = {
  'lx-field': [
    'flex flex-row items-start box-border w-full text-left',
    `gap-[${sp('sm')}]`,
    fs('sm'),
    color('on-surface'),
    p('font-family', 'inherit'),
  ].join(' '),
  'lx-field-size-normal': `${p('--lx-field-h', v('field-height'))} ${p('--lx-field-px', v('field-padding-x'))}`,
  'lx-field-size-dense': `${p('--lx-field-h', v('field-height-dense'))} ${p('--lx-field-px', v('field-padding-x-dense'))}`,
  'lx-field-labeled': p('--lx-field-top', v('field-label-space')),
  'lx-field-plain': p('--lx-field-top', '0px'),
  'lx-field-editable': 'cursor-text',
  'lx-field-readonly': 'cursor-default',
  'lx-field-disabled': `cursor-not-allowed ${p('opacity', v('field-disabled-opacity'))}`,
  'lx-field-placeholder-rest': p('--lx-field-placeholder-opacity', v('field-placeholder-rest-opacity')),
  'lx-field-placeholder-shown': p('--lx-field-placeholder-opacity', '1'),
  'lx-field-inner': 'relative flex flex-col flex-1 min-w-0 box-border pt-[var(--lx-field-top)]',
  'lx-field-side': `flex flex-none items-center self-start box-border mt-[var(--lx-field-top)] min-h-[var(--lx-field-h)] ${color('on-surface-muted')}`,
  'lx-field-box': `relative flex flex-row items-center box-border min-h-[var(--lx-field-h)] px-[var(--lx-field-px)] gap-[${sp('sm')}]`,
  ...fieldBoxes,
  'lx-field-control-container': 'relative flex flex-1 flex-row items-center min-w-0 self-stretch',
  'lx-field-marginal': `flex flex-none flex-row items-center gap-[${sp('xs')}] ${color('on-surface-muted')}`,
  'lx-field-prefix': `flex items-center shrink-0 pr-[${sp('xs')}] ${color('on-surface-muted')}`,
  'lx-field-suffix': `flex items-center shrink-0 pl-[${sp('xs')}] ${color('on-surface-muted')}`,
  'lx-field-native': [
    'block w-full min-w-0 m-0 p-0 border-0 bg-transparent outline-none',
    p('font', 'inherit'),
    p('color', 'inherit'),
    `placeholder:${p('opacity', 'var(--lx-field-placeholder-opacity)')}`,
    `placeholder:${color('on-surface-muted')}`,
    'disabled:cursor-not-allowed',
  ].join(' '),
  'lx-field-label-rest': fieldLabel('field-label-rest-transform', null),
  'lx-field-label-float': fieldLabel('field-label-float-transform', 'field-label-float-bg'),
  'lx-field-bottom': `flex items-start box-border pt-[${sp('xs')}] ${fs('xs')} ${color('on-surface-muted')}`,
  'lx-field-bottom-reserved': 'min-h-[var(--lx-field-bottom-height)]',
  'lx-field-bottom-free': 'min-h-0',
  'lx-field-hint': p('color', v('field-hint-color')),
  'lx-field-error': p('color', v('color-negative')),
  'lx-field-error-icon': `flex flex-none items-center ${color('negative')}`,
  // Rodapé: mensagens (hint/erro) ocupam o espaço; o contador fica à direita.
  'lx-field-messages': 'flex-1 min-w-0',
  'lx-field-counter': `flex-none ml-auto pl-[${sp('sm')}] leading-none ${p('color', v('field-counter-color'))}`,
  // `shadow_text`: sobreposto ao controle; o texto já digitado fica invisível (só ocupa o
  // espaço) e a sombra vem logo depois. Claro só com foco (como o Quasar).
  'lx-field-shadow-line': `absolute inset-0 flex items-center overflow-hidden whitespace-nowrap pointer-events-none ${p('font', 'inherit')} ${p('color', v('field-shadow-color'))}`,
  'lx-field-shadow-multiline': `absolute inset-0 flex items-start overflow-hidden whitespace-pre-wrap pointer-events-none py-[${sp('sm')}] ${p('overflow-wrap', 'break-word')} ${p('font', 'inherit')} ${p('color', v('field-shadow-color'))}`,
  'lx-field-shadow-idle': p('opacity', v('field-shadow-opacity')),
  'lx-field-shadow-focused': p('opacity', v('field-shadow-opacity-focused')),
  'lx-field-shadow-gap': p('visibility', 'hidden'),
}

// ── Input ─────────────────────────────────────────────────────────────────
const input: Shortcuts = {
  // `<textarea>` (junto de `lx-field-native`): com respiro em cima e embaixo; o redimensionamento
  // é um dos dois exclusivos abaixo.
  'lx-input-textarea': `py-[${sp('sm')}]`,
  'lx-input-textarea-resize': 'resize-y',
  // `autogrow`: o navegador mede (`field-sizing`); sem suporte, `autogrow.rs` grava o `height`.
  'lx-input-textarea-autogrow': [
    'box-border resize-none',
    p('field-sizing', 'content'),
    p('max-height', v('input-autogrow-max-height')),
  ].join(' '),
  // Botão de limpar (`clearable`): ícone nu, com anel de foco do teclado.
  'lx-input-clear': [
    'inline-flex items-center justify-center flex-none m-0 p-0 border-0 bg-transparent cursor-pointer',
    p('color', 'inherit'),
    p('border-radius', v('radius-sm')),
    'focus-visible:outline-2 focus-visible:outline-solid focus-visible:outline-offset-2',
    'focus-visible:[outline-color:var(--lx-tone)]',
  ].join(' '),
}

// ── Select ────────────────────────────────────────────────────────────────
// O campo é um `<input readonly>` (junto de `lx-field-native`); a lista usa o popup `lx-menu`.
// As opções reaproveitam os tokens de `Item` (`item-hover-bg`, `item-active-*`). Estados da opção
// (ativa × selecionada × desabilitada) em shortcuts completos; a base só tem layout.
const selectOutline = `${p('outline', `2px solid ${v('color-primary')}`)} ${p('outline-offset', '-2px')}`
const select: Shortcuts = {
  'lx-select-input': [
    'cursor-pointer overflow-hidden text-ellipsis whitespace-nowrap',
    p('caret-color', 'transparent'),
  ].join(' '),
  'lx-select-arrow-closed': `flex flex-none items-center ${color('on-surface-muted')} ${p('transition', 'transform 150ms')} motion-reduce:${p('transition', 'none')}`,
  'lx-select-arrow-open': `flex flex-none items-center ${color('on-surface-muted')} ${p('transition', 'transform 150ms')} ${p('transform', 'rotate(180deg)')} motion-reduce:${p('transition', 'none')}`,
  'lx-select-list': `flex flex-col box-border m-0 list-none py-[${sp('xs')}] px-0`,
  'lx-select-option-label': 'flex-1 min-w-0',
  'lx-select-option-check': 'flex flex-none items-center',
  // Caixa de busca (`use_filter`): fixa no topo do popup (que rola inteiro), com a cor do menu.
  'lx-select-filter-box': `sticky top-0 z-10 box-border p-[${sp('sm')}] ${p('background', v('menu-bg'))} ${p('border-bottom', v('menu-border'))}`,
  'lx-select-filter': [
    'block w-full min-w-0 box-border m-0',
    `px-[${sp('sm')}] py-[${sp('xs')}]`,
    p('font', 'inherit'),
    p('color', 'inherit'),
    p('background', v('field-bg')),
    p('border', `${v('border-width')} solid ${v('field-border')}`),
    p('border-radius', v('field-radius')),
    p('outline', 'none'),
    `focus:${p('border-color', v('color-primary'))}`,
    `focus:${p('box-shadow', `0 0 0 ${v('field-focus-ring-width')} ${v('color-primary')}`)}`,
    `placeholder:${color('on-surface-muted')}`,
  ].join(' '),
  'lx-select-empty': `px-[${v('item-padding-x')}] py-[${sp('md')}] ${color('on-surface-muted')}`,
  'lx-select-option-base': `flex flex-row items-center box-border gap-[${sp('md')}] min-h-[2.5rem] px-[${v('item-padding-x')}] py-[${sp('xs')}] list-none`,
  'lx-select-option-idle': `lx-select-option-base cursor-pointer ${color('on-surface')}`,
  'lx-select-option-active': `lx-select-option-base cursor-pointer ${p('background', v('item-hover-bg'))} ${color('on-surface')} ${selectOutline}`,
  'lx-select-option-selected': `lx-select-option-base cursor-pointer ${p('background', v('item-active-bg'))} ${p('color', v('item-active-color'))}`,
  'lx-select-option-selected-active': `lx-select-option-base cursor-pointer ${p('background', v('item-active-bg'))} ${p('color', v('item-active-color'))} ${selectOutline}`,
  'lx-select-option-disabled': `lx-select-option-base cursor-not-allowed ${color('on-surface-muted')} ${p('opacity', '0.6')}`,
}

// ── BtnToggle ─────────────────────────────────────────────────────────────
// Grupo de botões com uma opção ativa. Três designs; o item tem estados exclusivos
// (ativo × inativo × desabilitado) como shortcuts COMPLETOS, e a base do item não define cor,
// fundo nem raio. Cor "do tema" (sem `color`/`toggle_color`) lê tokens `--lx-btn-toggle-*`
// (no scandi, o aspecto "segmented": trilho `surface-muted`, ativo em `surface` com sombra);
// com cor, o item lê `--lx-tone*` (o grupo carrega a cor inativa; o item ativo, a sua).
const btRadius = (outer: string, inner: string) => ({
  only: `${outer}`,
  first: `${outer} ${inner} ${inner} ${outer}`,
  middle: `${inner}`,
  last: `${inner} ${outer} ${outer} ${inner}`,
})
const btnTogglePositions: Shortcuts = Object.fromEntries(
  (['normal', 'rounded'] as const).flatMap((kind) => {
    const suffix = kind === 'rounded' ? '-rounded' : ''
    const radii = btRadius(v(`btn-toggle-radius${suffix}`), v(`btn-toggle-radius-inner${suffix}`))
    return Object.entries(radii).map(([pos, r]) => [
      `lx-btn-toggle-pos-${pos}${suffix}`,
      p('border-radius', r),
    ])
  }),
)
const btItem = (opts: {
  bg: string
  color: string
  border: string
  shadow?: string
  hover?: string
  disabled: boolean
}): string =>
  [
    p('background', opts.bg),
    p('color', opts.color),
    p('border-color', opts.border),
    opts.shadow ? p('box-shadow', opts.shadow) : '',
    opts.disabled
      ? `pointer-events-none cursor-not-allowed ${p('opacity', v('state-disabled-opacity'))}`
      : `cursor-pointer ${opts.hover ?? ''}`,
  ]
    .filter(Boolean)
    .join(' ')
const btnToggleItems: Shortcuts = {}
for (const disabled of [false, true]) {
  const d = disabled ? '-disabled' : ''
  const hoverSoft = `hover:${p('background', v('tone-soft'))}`
  // unelevated: tema (tokens) ou cor (tone)
  btnToggleItems[`lx-btn-toggle-item-unelevated-off${d}`] = btItem({
    bg: 'transparent', color: v('btn-toggle-off-color'), border: 'transparent', disabled,
    hover: `hover:${p('color', v('btn-toggle-off-hover-color'))}`,
  })
  btnToggleItems[`lx-btn-toggle-item-unelevated-off-tone${d}`] = btItem({
    bg: 'transparent', color: v('tone'), border: 'transparent', disabled, hover: hoverSoft,
  })
  btnToggleItems[`lx-btn-toggle-item-unelevated-on${d}`] = btItem({
    bg: v('btn-toggle-on-bg'), color: v('btn-toggle-on-color'), border: 'transparent',
    shadow: v('btn-toggle-on-shadow'), disabled,
  })
  btnToggleItems[`lx-btn-toggle-item-unelevated-on-tone${d}`] = btItem({
    bg: v('tone'), color: v('tone-contrast'), border: v('tone'), disabled,
  })
  btnToggleItems[`lx-btn-toggle-item-outline-off${d}`] = btItem({
    bg: 'transparent', color: v('tone'), border: v('tone'), disabled, hover: hoverSoft,
  })
  btnToggleItems[`lx-btn-toggle-item-outline-on${d}`] = btItem({
    bg: v('tone'), color: v('tone-contrast'), border: v('tone'), disabled,
  })
  btnToggleItems[`lx-btn-toggle-item-flat-off${d}`] = btItem({
    bg: 'transparent', color: v('tone'), border: 'transparent', disabled, hover: hoverSoft,
  })
  btnToggleItems[`lx-btn-toggle-item-flat-on${d}`] = btItem({
    bg: v('tone-soft'), color: v('tone'), border: 'transparent', disabled,
  })
}
const trackBox = (radius: string) =>
  [
    p('background', v('btn-toggle-track-bg')),
    p('border', `${v('border-width')} solid ${v('color-border')}`),
    p('padding', v('btn-toggle-track-padding')),
    p('gap', v('btn-toggle-track-gap')),
    p('border-radius', v(radius)),
  ].join(' ')
const btnToggle: Shortcuts = {
  // Grupo. `fit` × `spread` (largura); o design define trilho/colagem; raios e `gap` só nas variantes.
  'lx-btn-toggle': 'box-border max-w-full align-middle',
  'lx-btn-toggle-fit': 'inline-flex',
  'lx-btn-toggle-spread': 'flex w-full [&>*]:flex-[1_1_0%] [&>*]:min-w-0',
  // unelevated: trilho (no scandi, `surface-muted` com borda fina)
  'lx-btn-toggle-track': trackBox('btn-toggle-track-radius'),
  'lx-btn-toggle-track-rounded': trackBox('btn-toggle-track-radius-rounded'),
  // outline: botões colados, bordas sobrepostas; flat: colados, sem borda
  'lx-btn-toggle-joined-outline': 'gap-0 [&>*+*]:-ml-px',
  'lx-btn-toggle-joined-flat': 'gap-0',
  // Item (sem cor, fundo nem raio: vêm do estado e da posição)
  'lx-btn-toggle-item': [
    'relative inline-flex items-center justify-center box-border no-underline select-none whitespace-nowrap',
    `gap-[${sp('sm')}] border border-solid`,
    fs('sm'),
    p('font-family', 'inherit'),
    p('font-weight', v('btn-weight')),
    p('text-transform', v('btn-transform')),
    p('letter-spacing', v('btn-tracking')),
    `transition-[color,background-color,box-shadow] duration-[${v('duration-fast')}]`,
    'focus-visible:outline-2 focus-visible:outline-solid focus-visible:outline-offset-2 focus-visible:z-[1]',
    'focus-visible:[outline-color:var(--lx-tone,var(--lx-color-primary))]',
  ].join(' '),
  'lx-btn-toggle-size-normal': `h-[${v('control-height')}] px-[${sp('md')}]`,
  'lx-btn-toggle-size-dense': `h-[${v('control-height-sm')}] px-[${sp('sm')}]`,
  ...btnTogglePositions,
  ...btnToggleItems,
}

// ── OptionGroup ───────────────────────────────────────────────────────────
// Native `<input type=radio|checkbox>`. Radio and checkbox keep the native look (`accent-color` =
// the tone); the toggle is an `appearance:none` track whose thumb is a gradient moved by
// `background-position`. The tone comes from `lx-tone-*` on the group. Exclusive states are
// complete shortcuts.
const optionFocus = [
  'focus-visible:outline-2 focus-visible:outline-solid focus-visible:outline-offset-2',
  'focus-visible:[outline-color:var(--lx-tone)]',
].join(' ')
const optionControl = (size: string) =>
  [
    'flex-none m-0 cursor-pointer',
    p('width', v(size)),
    p('height', v(size)),
    p('accent-color', 'var(--lx-tone)'),
    optionFocus,
  ].join(' ')
const toggleBase = (w: string, h: string) =>
  [
    'flex-none m-0 box-border cursor-pointer appearance-none rounded-full bg-no-repeat',
    p('width', v(w)),
    p('height', v(h)),
    p('padding', v('option-toggle-pad')),
    p('background-origin', 'content-box'),
    p('background-size', `calc(${v(h)} - 2 * ${v('option-toggle-pad')}) 100%`),
    p(
      'transition',
      `background-color ${v('duration-fast')} ${v('easing')}, background-position ${v('duration-fast')} ${v('easing')}`,
    ),
    `motion-reduce:${p('transition', 'none')}`,
    optionFocus,
  ].join(' ')
const thumb = p(
  'background-image',
  `radial-gradient(circle closest-side, ${v('option-toggle-thumb')} 96%, transparent 100%)`,
)
const optionGroup: Shortcuts = {
  'lx-option-group-col': `flex flex-col ${p('gap', v('option-gap'))}`,
  'lx-option-group-col-dense': `flex flex-col ${p('gap', v('option-gap-dense'))}`,
  'lx-option-group-inline': `flex flex-row flex-wrap items-center ${p('gap', v('option-gap-inline'))}`,
  'lx-option-group-inline-dense': `flex flex-row flex-wrap items-center ${p('gap', v('option-gap-dense'))}`,
  'lx-option-item': `inline-flex items-center cursor-pointer ${p('gap', v('option-item-gap'))} ${p('color', v('option-label-color'))}`,
  'lx-option-item-left': `inline-flex flex-row-reverse items-center justify-end cursor-pointer ${p('gap', v('option-item-gap'))} ${p('color', v('option-label-color'))}`,
  'lx-option-item-disabled': `inline-flex items-center cursor-not-allowed ${p('gap', v('option-item-gap'))} ${p('color', v('option-label-color'))} ${p('opacity', v('option-disabled-opacity'))}`,
  'lx-option-item-left-disabled': `inline-flex flex-row-reverse items-center justify-end cursor-not-allowed ${p('gap', v('option-item-gap'))} ${p('color', v('option-label-color'))} ${p('opacity', v('option-disabled-opacity'))}`,
  'lx-option-label': 'select-none',
  'lx-radio': optionControl('option-size'),
  'lx-radio-dense': optionControl('option-size-dense'),
  'lx-checkbox': optionControl('option-size'),
  'lx-checkbox-dense': optionControl('option-size-dense'),
  'lx-toggle-off': [
    toggleBase('option-toggle-width', 'option-toggle-height'),
    p('background-color', v('option-toggle-off-bg')),
    thumb,
    p('background-position', 'left center'),
  ].join(' '),
  'lx-toggle-on': [
    toggleBase('option-toggle-width', 'option-toggle-height'),
    p('background-color', 'var(--lx-tone)'),
    thumb,
    p('background-position', 'right center'),
  ].join(' '),
  'lx-toggle-off-dense': [
    toggleBase('option-toggle-width-dense', 'option-toggle-height-dense'),
    p('background-color', v('option-toggle-off-bg')),
    thumb,
    p('background-position', 'left center'),
  ].join(' '),
  'lx-toggle-on-dense': [
    toggleBase('option-toggle-width-dense', 'option-toggle-height-dense'),
    p('background-color', 'var(--lx-tone)'),
    thumb,
    p('background-position', 'right center'),
  ].join(' '),
}

export const shortcuts: Shortcuts = {
  ...tones,
  ...field,
  ...input,
  ...select,

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

  ...btnToggle,
  ...optionGroup,

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

  // ── Spinner ───────────────────────────────────────────────────────────────
  // Sem cor própria: herda `currentColor` (cabe no append do Field e dentro do Btn).
  'lx-spinner': `inline-block align-middle shrink-0`,
  'lx-spinner-tone': p('color', v('tone')),
  'lx-spinner-path': [
    p('stroke-dasharray', '1, 200'),
    p('stroke-dashoffset', '0'),
    p('transform-origin', '25px 25px'),
    p(
      'animation',
      `lx-spinner-rotate ${v('spinner-rotate-duration')} linear infinite, lx-spinner-dash ${v('spinner-dash-duration')} ease-in-out infinite`,
    ),
    // reduced-motion: mais lento, não parado
    `motion-reduce:${p('animation-duration', `${v('spinner-rotate-duration-reduced')}, ${v('spinner-dash-duration-reduced')}`)}`,
  ].join(' '),

  // ── LinearProgress ────────────────────────────────────────────────────────
  // Largura total do pai e altura própria (inline): não depende de altura do pai.
  // Raio só nas variantes (rounded/square); direção (start/end, motion/segments) também.
  'lx-linear-progress': 'block relative w-full overflow-hidden shrink-0 box-border',
  'lx-linear-progress-rounded': p('border-radius', v('linear-progress-radius')),
  'lx-linear-progress-square': 'rounded-none',
  'lx-linear-progress-layer': 'absolute inset-0',
  'lx-linear-progress-track': p('background', v('linear-progress-track-bg')),
  'lx-linear-progress-track-tone': `${p('background', v('tone'))} ${p('opacity', v('linear-progress-track-tone-opacity'))}`,
  'lx-linear-progress-bar': `absolute top-0 bottom-0 ${p('background', v('tone'))}`,
  'lx-linear-progress-buffer': `absolute top-0 bottom-0 ${p('background', v('tone'))} ${p('opacity', v('linear-progress-buffer-opacity'))}`,
  'lx-linear-progress-stripe': [
    'absolute top-0 bottom-0',
    p('background-image', v('linear-progress-stripe-image')),
    p('background-size', `${v('linear-progress-stripe-size')} ${v('linear-progress-stripe-size')}`),
  ].join(' '),
  'lx-linear-progress-start': 'left-0',
  'lx-linear-progress-end': 'right-0',
  'lx-linear-progress-animated': [
    p('transition-property', 'width'),
    p('transition-duration', v('linear-progress-transition-duration')),
    p('transition-timing-function', v('linear-progress-transition-easing')),
    `motion-reduce:${p('transition-duration', '0s')}`,
  ].join(' '),
  'lx-linear-progress-segments': 'absolute inset-0 flex flex-row',
  'lx-linear-progress-segments-reverse': 'absolute inset-0 flex flex-row-reverse',
  'lx-linear-progress-segment': `h-full shrink-0 ${p('background', v('tone'))}`,
  // indeterminado/query: duas barras de largura total animadas; reverse espelha o conjunto
  'lx-linear-progress-motion': 'absolute inset-0 overflow-hidden',
  'lx-linear-progress-motion-reverse': `absolute inset-0 overflow-hidden ${p('scale', '-1 1')}`,
  'lx-linear-progress-indeterminate': [
    'absolute inset-0',
    p('background', v('tone')),
    p('transform-origin', '0 0'),
    p('animation', 'lx-linear-progress-indeterminate var(--lx-linear-progress-speed) cubic-bezier(.65,.815,.735,.395) infinite'),
    // reduced-motion: mais lento (3x), não parado
    `motion-reduce:${p('animation-duration', 'calc(var(--lx-linear-progress-speed) * 3)')}`,
  ].join(' '),
  'lx-linear-progress-indeterminate-short': [
    'absolute inset-0',
    p('background', v('tone')),
    p('transform-origin', '0 0'),
    p('transform', 'translateX(-101%)'),
    p('animation', 'lx-linear-progress-indeterminate-short var(--lx-linear-progress-speed) cubic-bezier(.165,.84,.44,1) infinite'),
    p('animation-delay', 'calc(var(--lx-linear-progress-speed) * 0.55)'),
    `motion-reduce:${p('animation-duration', 'calc(var(--lx-linear-progress-speed) * 3)')}`,
    `motion-reduce:${p('animation-delay', 'calc(var(--lx-linear-progress-speed) * 1.65)')}`,
  ].join(' '),

  // ── EmptyState ────────────────────────────────────────────────────────────
  'lx-empty-state': [
    'flex flex-col items-center justify-center text-center box-border',
    `gap-[${sp('sm')}]`,
    p('padding', v('empty-state-padding')),
    fs('sm'),
    p('color', v('empty-state-hint')),
  ].join(' '),
  'lx-empty-state-icon': [
    p('font-size', v('empty-state-icon-size')),
    p('color', v('empty-state-icon')),
  ].join(' '),
  'lx-empty-state-title': [
    fs('md'),
    p('font-weight', v('empty-state-title-weight')),
    p('color', v('empty-state-title')),
  ].join(' '),
  'lx-empty-state-hint': `${p('color', v('empty-state-hint'))} max-w-[${v('empty-state-hint-max-width')}]`,
  'lx-empty-state-actions': `flex flex-wrap items-center justify-center gap-[${sp('sm')}] mt-[${sp('sm')}]`,

  // ── DetailList ────────────────────────────────────────────────────────────
  // `<dl>` de linhas `<div><dt/><dd/></div>`. O layout, a densidade e o separador são shortcuts
  // completos e exclusivos; a linha é um flex que quebra quando o contêiner é estreito (o valor
  // desce para baixo do termo), sem depender da largura da janela.
  'lx-detail-list': [
    'block m-0 box-border',
    fs('sm'),
    color('on-surface'),
  ].join(' '),
  'lx-detail-list-horizontal': [
    '[&>div]:flex [&>div]:flex-wrap',
    `[&>div]:[column-gap:${sp('lg')}]`,
    `[&>div]:[row-gap:${sp('xs')}]`,
    `[&_dt]:${p('flex', `0 0 ${v('detail-list-term-width')}`)}`,
    '[&_dt]:max-w-full',
    `[&_dd]:${p('flex', `1 1 ${v('detail-list-value-min-width')}`)}`,
  ].join(' '),
  'lx-detail-list-vertical': [
    '[&>div]:flex [&>div]:flex-col',
    `[&>div]:[row-gap:${sp('xs')}]`,
  ].join(' '),
  'lx-detail-list-normal': `[&>div]:${p('padding-block', v('detail-list-row-padding'))}`,
  'lx-detail-list-dense': `[&>div]:${p('padding-block', v('detail-list-row-padding-dense'))}`,
  'lx-detail-list-separated': `[&>div+div]:${p('border-top', v('detail-list-separator'))}`,
  'lx-detail-term': `m-0 min-w-0 break-words ${color('on-surface-muted')}`,
  'lx-detail-value': 'm-0 min-w-0 break-words',

  // ── PageHeader ────────────────────────────────────────────────────────────
  // Texto cresce; ações ficam à direita e, se não couberem, descem (flex-wrap).
  'lx-page-header': [
    'flex flex-wrap items-center box-border',
    `gap-[${sp('md')}]`,
    p('margin-bottom', v('page-header-margin')),
  ].join(' '),
  'lx-page-header-text': 'flex flex-col grow basis-0 min-w-[min(100%,12rem)]',
  'lx-page-header-title': [
    'm-0',
    p('font-size', v('page-header-title-size')),
    p('font-weight', v('page-header-title-weight')),
    p('line-height', v('page-header-title-line-height')),
    p('color', v('page-header-title-color')),
  ].join(' '),
  'lx-page-header-subtitle': [
    'm-0',
    fs('sm'),
    p('color', v('page-header-subtitle-color')),
  ].join(' '),
  'lx-page-header-actions': `flex flex-wrap items-center gap-[${sp('sm')}]`,

  // ── Stat ──────────────────────────────────────────────────────────────────
  'lx-stat-grid': [
    'grid',
    p('grid-template-columns', `repeat(auto-fit, minmax(min(100%, ${v('stat-grid-min-width')}), 1fr))`),
    p('gap', v('stat-grid-gap')),
  ].join(' '),
  'lx-stat': `flex items-start justify-between box-border min-w-0 gap-[${sp('md')}]`,
  'lx-stat-body': `flex flex-col min-w-0 gap-[${sp('xs')}]`,
  'lx-stat-label': [
    fs('sm'),
    p('color', v('stat-label-color')),
    p('font-weight', v('stat-label-weight')),
  ].join(' '),
  'lx-stat-value': [
    p('font-size', v('stat-value-size')),
    p('font-weight', v('stat-value-weight')),
    p('line-height', v('stat-value-line-height')),
    'break-words',
  ].join(' '),
  'lx-stat-value-plain': p('color', v('stat-value-color')),
  'lx-stat-value-tone': p('color', 'var(--lx-tone)'),
  'lx-stat-hint': [fs('xs'), p('color', v('stat-hint-color'))].join(' '),
  'lx-stat-icon': `inline-flex shrink-0 ${p('font-size', v('stat-icon-size'))} ${p('line-height', '1')}`,
  'lx-stat-icon-plain': p('color', v('stat-icon-color')),
  'lx-stat-icon-tone': p('color', 'var(--lx-tone)'),

  // ── Dialog ────────────────────────────────────────────────────────────────
  // `<dialog>` nativo ocupa a janela inteira e é transparente; o painel (`lx-dialog-body`) é o
  // que se vê. Posição, tamanho e estado (entrada/saída) são shortcuts completos e exclusivos.
  'lx-dialog': [
    'fixed inset-0 m-0 p-0 border-0 box-border overflow-hidden w-full h-full max-w-none max-h-none',
    p('background', 'transparent'),
    p('color', 'inherit'),
    `backdrop:${p('background', v('dialog-scrim'))}`,
    `backdrop:${p('backdrop-filter', v('dialog-backdrop'))}`,
    `backdrop:${p('transition', `opacity ${v('dialog-duration')} ${v('dialog-easing')}`)}`,
  ].join(' '),
  'lx-dialog-scrim-shown': `backdrop:${p('opacity', '1')}`,
  'lx-dialog-scrim-hidden': `backdrop:${p('opacity', '0')}`,
  ...dialogInner,
  'lx-dialog-body': [
    'pointer-events-auto overflow-auto box-border',
    p('background', v('dialog-bg')),
    p('color', v('dialog-color')),
    p('box-shadow', v('dialog-shadow')),
    p(
      'transition',
      `opacity ${v('dialog-duration')} ${v('dialog-easing')}, transform ${v('dialog-duration')} ${v('dialog-easing')}`,
    ),
  ].join(' '),
  ...dialogBody,
  'lx-dialog-body-max': [
    'w-full h-full',
    p('max-width', '100vw'),
    p('max-height', '100dvh'),
    'rounded-none',
  ].join(' '),
  'lx-dialog-w-normal': p('max-width', `min(100%, ${v('dialog-max-width')})`),
  'lx-dialog-w-full': 'w-full max-w-full',
  'lx-dialog-h-pad': p('max-height', `calc(100dvh - 2 * ${v('dialog-padding')})`),
  'lx-dialog-h-edge': p('max-height', '100dvh'),
  'lx-dialog-h-full': 'h-full max-h-full',
  'lx-dialog-open': `${p('opacity', '1')} ${p('transform', 'none')}`,
  ...dialogEnter,
  // "sacudir" do persistent: a animação termina sozinha (`animationend` limpa a classe)
  'lx-dialog-shake': [
    p('animation', `lx-dialog-shake ${v('dialog-shake-duration')} ease-in-out`),
    `motion-reduce:${p('animation', 'none')}`,
  ].join(' '),
  // Dialog plugin (`Plugins::dialog`): the content of the panel, over a flat `Card`.
  'lx-dialog-plugin': `${p('width', v('dialog-plugin-width'))} max-w-full box-border`,
  'lx-dialog-title': `${p('font-size', v('dialog-plugin-title-size'))} ${p('font-weight', v('dialog-plugin-title-weight'))} ${p('line-height', '1.3')}`,
  'lx-dialog-message': `${p('white-space', 'pre-line')} ${p('overflow-wrap', 'anywhere')}`,
  'lx-dialog-progress': `flex justify-center ${p('padding', v('dialog-plugin-gap') + ' 0')}`,
  'lx-dialog-form': `flex flex-col ${p('gap', v('dialog-plugin-gap'))}`,
  // ── Notify plugin (`Plugins::notify`) ─────────────────────────────────────
  // A camada é um `popover="manual"` que cobre a janela (sem `display`: o UA esconde o popover
  // fechado com `display:none`); as pilhas são uma por posição e só os toasts recebem o ponteiro.
  'lx-notify-layer': [
    'fixed inset-0 m-0 p-0 box-border w-full h-full overflow-visible pointer-events-none',
    p('border', '0'),
    p('background', 'transparent'),
    p('color', 'inherit'),
    p('max-width', 'none'),
    p('max-height', 'none'),
  ].join(' '),
  'lx-notify-stack': [
    'absolute flex box-border pointer-events-none',
    p('gap', v('notify-gap')),
    p('padding', v('notify-offset')),
  ].join(' '),
  ...notifyStacks,
  'lx-notify-item': [
    'relative flex items-center box-border pointer-events-auto',
    p('font-size', v('notify-font-size')),
    p('border-radius', v('notify-radius')),
    p('box-shadow', v('notify-shadow')),
    p('min-width', v('notify-min-width')),
    p('max-width', v('notify-max-width')),
    p('transition', `opacity ${v('notify-duration')} ${v('notify-easing')}, transform ${v('notify-duration')} ${v('notify-easing')}`),
    `motion-reduce:${p('transition', 'none')}`,
  ].join(' '),
  'lx-notify-plain': `${p('background', v('notify-bg'))} ${p('color', v('notify-color'))}`,
  'lx-notify-toned': `${p('background', v('tone'))} ${p('color', v('tone-contrast'))}`,
  'lx-notify-single': `${p('padding', v('notify-padding'))} ${p('min-height', v('notify-min-height'))}`,
  'lx-notify-multi': `flex-wrap ${p('padding', v('notify-padding-multi'))} ${p('min-height', v('notify-min-height-multi'))}`,
  // Estados exclusivos (entrada/saída × aberto); a entrada vem de cima nas posições do topo.
  'lx-notify-enter-up': `${p('opacity', '0')} ${p('transform', 'translateY(calc(-1 * var(--lx-notify-slide)))')} motion-reduce:${p('transform', 'none')}`,
  'lx-notify-enter-down': `${p('opacity', '0')} ${p('transform', 'translateY(var(--lx-notify-slide))')} motion-reduce:${p('transform', 'none')}`,
  'lx-notify-open': `${p('opacity', '1')} ${p('transform', 'none')}`,
  'lx-notify-icon': `shrink-0 inline-flex ${p('font-size', v('notify-icon-size'))} ${p('margin-right', v('notify-gap'))}`,
  'lx-notify-avatar': `shrink-0 object-cover ${p('width', v('notify-avatar-size'))} ${p('height', v('notify-avatar-size'))} ${p('border-radius', '50%')} ${p('margin-right', v('notify-gap'))}`,
  // `flex: 1 1 auto` (não `1 1 0%`): no multi-line, a base 0 deixaria as ações caberem na mesma linha
  'lx-notify-text': `min-w-0 ${p('flex', '1 1 auto')} ${p('padding', v('notify-text-padding'))}`,
  'lx-notify-message': p('overflow-wrap', 'anywhere'),
  'lx-notify-caption': `${p('font-size', '0.9em')} ${p('opacity', '0.7')} ${p('overflow-wrap', 'anywhere')}`,
  'lx-notify-actions': `flex items-center shrink-0 ${p('margin-left', v('notify-gap'))} ${p('gap', v('notify-action-gap'))}`,
  'lx-notify-actions-multi': `${p('flex-basis', '100%')} justify-end ${p('margin-left', '0')}`,
  'lx-notify-action': [
    'inline-flex items-center cursor-pointer',
    p('gap', v('notify-action-gap')),
    p('padding', v('notify-action-padding')),
    p('border', '0'),
    p('background', 'transparent'),
    p('color', 'inherit'),
    p('font', 'inherit'),
    p('font-weight', '600'),
    p('border-radius', v('notify-action-radius')),
    `hover:${p('background', 'color-mix(in srgb, currentColor 14%, transparent)')}`,
  ].join(' '),
  'lx-notify-action-tone': p('color', v('tone')),
  'lx-notify-badge': [
    'absolute',
    p('padding', '0.2rem 0.5rem'),
    p('border-radius', v('notify-radius')),
    p('font-size', v('font-size-xs')),
    p('line-height', '1'),
    p('background', v('color-negative')),
    p('color', v('color-on-negative')),
    p('box-shadow', v('notify-shadow')),
    p('top', '-0.5rem'),
  ].join(' '),
  'lx-notify-badge-start': p('left', '-0.75rem'),
  'lx-notify-badge-end': p('right', '-0.75rem'),
  'lx-notify-progress': [
    'absolute inset-x-0 pointer-events-none',
    p('bottom', '0'),
    p('height', '3px'),
    p('background', 'currentColor'),
    p('opacity', '0.3'),
    p('border-radius', '0 0 var(--lx-notify-radius) var(--lx-notify-radius)'),
    p('transform-origin', '0 50%'),
    p('transform', 'scaleX(0)'),
    p('animation', 'lx-notify-progress var(--lx-notify-progress-duration) linear forwards'),
  ].join(' '),
  // Bloqueio de rolagem do <html> (a classe é posta pelo componente, não vem de markup)
  'lx-scroll-lock': `overflow-hidden ${p('scrollbar-gutter', 'stable')}`,

  // ── Menu e Tooltip ────────────────────────────────────────────────────────
  // Popups `popover="manual"` (camada superior); `top`/`left` vêm do motor de posição, em estilo
  // inline. NÃO definir `display`: o UA esconde `[popover]:not(:popover-open)` com `display:none`.
  // Estado (entrada/aberto) em shortcuts completos e exclusivos.
  'lx-menu': [
    'fixed m-0 p-0 box-border w-max inset-auto overflow-x-hidden overflow-y-auto pointer-events-auto',
    p('outline', '0'),
    p('background', v('menu-bg')),
    p('color', v('menu-color')),
    p('border', v('menu-border')),
    p('border-radius', v('menu-radius')),
    p('box-shadow', v('menu-shadow')),
    p('max-width', v('menu-max-width')),
    p('max-height', v('menu-max-height')),
    p('transform-origin', 'top'),
    fs('sm'),
    p(
      'transition',
      `opacity ${v('menu-duration')} ${v('menu-easing')}, transform ${v('menu-duration')} ${v('menu-easing')}`,
    ),
  ].join(' '),
  'lx-menu-enter': `${p('opacity', '0')} ${p('transform', `scale(${v('menu-scale-from')})`)} motion-reduce:${p('transform', 'none')}`,
  'lx-menu-open': `${p('opacity', '1')} ${p('transform', 'none')}`,
  'lx-tooltip': [
    'fixed m-0 box-border w-max inset-auto overflow-hidden border-0 pointer-events-none',
    p('background', v('tooltip-bg')),
    p('color', v('tooltip-color')),
    p('border-radius', v('tooltip-radius')),
    p('padding', v('tooltip-padding')),
    p('font-size', v('tooltip-font-size')),
    p('box-shadow', v('tooltip-shadow')),
    p('max-width', v('tooltip-max-width')),
    p('max-height', v('tooltip-max-height')),
    p('transition', `opacity ${v('tooltip-duration')} ${v('tooltip-easing')}`),
  ].join(' '),
  'lx-tooltip-enter': p('opacity', '0'),
  'lx-tooltip-open': p('opacity', '1'),

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
  'lx-item-clickable': `cursor-pointer ${p('transition', 'background-color 150ms')} hover:${p('background', v('item-hover-bg'))} focus-visible:${p('outline-offset', '-2px')}`,
  'lx-item-active': `cursor-pointer ${p('background', v('item-active-bg'))} ${p('color', v('item-active-color'))}`,
  'lx-item-section-main': 'flex flex-col justify-center flex-1 min-w-0',
  'lx-item-section-avatar': 'flex flex-col justify-center flex-none min-w-[1.5rem] items-start',
  'lx-item-section-side': `flex flex-col justify-center flex-none items-end ${color('on-surface-muted')}`,
  'lx-item-label': 'block',
  // Espaço acima separa os grupos; o primeiro grupo não leva espaço extra.
  'lx-item-label-header': `block px-[${v('item-padding-x')}] pt-[${sp('lg')}] pb-[${sp('xs')}] first:pt-[${sp('sm')}] ${fs('xs')} ${color('on-surface-muted')} ${p('font-weight', v('font-weight-strong'))}`,
  'lx-item-label-caption': `block ${fs('xs')} ${color('on-surface-muted')}`,
  'lx-item-label-clamp': 'overflow-hidden [display:-webkit-box] [-webkit-box-orient:vertical]',

  // ── ExpansionItem ────────────────────────────────────────────────────────
  // Estados exclusivos como shortcuts completos. O conteúdo anima `grid-template-rows` (0fr → 1fr)
  // e o fechado fica `visibility: hidden` (a visibilidade só muda ao fim da saída, por isso o atraso).
  'lx-expansion-item': 'block box-border',
  'lx-expansion-item-sep-none': `${p('border-top', '0 solid transparent')}`,
  'lx-expansion-item-sep-closed': `${p('border-top', `${v('border-width')} solid transparent`)} ${p('border-bottom', `${v('border-width')} solid transparent`)}`,
  'lx-expansion-item-sep-open': `${border('top')} ${border('bottom')}`,
  'lx-expansion-header-enabled': 'block',
  'lx-expansion-header-disabled': `block pointer-events-none cursor-not-allowed ${p('opacity', v('state-disabled-opacity'))}`,
  'lx-expansion-icon-closed': `flex items-center ${p('transition', `transform ${v('expansion-item-duration')} ${v('expansion-item-easing')}`)} motion-reduce:${p('transition', 'none')}`,
  'lx-expansion-icon-open': `flex items-center ${p('transform', 'rotate(180deg)')} ${p('transition', `transform ${v('expansion-item-duration')} ${v('expansion-item-easing')}`)} motion-reduce:${p('transition', 'none')}`,
  'lx-expansion-content-closed': `grid ${p('grid-template-rows', '0fr')} ${p('visibility', 'hidden')} ${p('transition', `grid-template-rows ${v('expansion-item-duration')} ${v('expansion-item-easing')}, visibility 0s linear ${v('expansion-item-duration')}`)} motion-reduce:${p('transition', 'none')}`,
  'lx-expansion-content-open': `grid ${p('grid-template-rows', '1fr')} ${p('visibility', 'visible')} ${p('transition', `grid-template-rows ${v('expansion-item-duration')} ${v('expansion-item-easing')}, visibility 0s linear 0s`)} motion-reduce:${p('transition', 'none')}`,
  'lx-expansion-content-clip': 'min-h-0 overflow-hidden',

  // ── Layout / Header / Toolbar / Drawer / Page ─────────────────────────────
  'lx-layout': [
    'grid min-h-screen box-border',
    p('grid-template-columns', 'auto minmax(0,1fr) auto'),
    p('grid-template-rows', 'auto 1fr auto'),
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
  'lx-footer': [
    'sticky bottom-0 z-30 box-border',
    p('grid-column', '1/-1'),
    p('grid-row', '3'),
    p('min-height', `calc(${v('footer-height')} + env(safe-area-inset-bottom,0px))`),
    p('padding-bottom', 'env(safe-area-inset-bottom,0px)'),
    p('background', v('footer-bg')),
    p('color', v('footer-color')),
    p('backdrop-filter', v('footer-backdrop')),
    `[&_.lx-toolbar]:${p('min-height', v('footer-height'))}`,
  ].join(' '),
  'lx-footer-bordered': p('border-top', `${v('border-width')} solid ${v('footer-border')}`),
  'lx-footer-elevated': p('box-shadow', v('footer-shadow')),
  'lx-bottom-bar': [
    'fixed bottom-0 inset-x-0 z-30 box-border flex items-center',
    `px-[${sp('md')}]`,
    p('min-height', `calc(${v('bottom-bar-height')} + env(safe-area-inset-bottom,0px))`),
    p('padding-bottom', 'env(safe-area-inset-bottom,0px)'),
    p('background', v('bottom-bar-bg')),
    p('color', v('bottom-bar-color')),
    p('backdrop-filter', v('bottom-bar-backdrop')),
  ].join(' '),
  'lx-bottom-bar-bordered': p('border-top', `${v('border-width')} solid ${v('bottom-bar-border')}`),
  'lx-bottom-bar-elevated': p('box-shadow', v('bottom-bar-shadow')),
  'lx-bottom-bar-inner': `mx-auto w-full flex items-center justify-between gap-[${sp('sm')}] max-w-[${v('bottom-bar-max-width')}]`,
  'lx-bottom-bar-space': p('padding-bottom', `calc(${v('bottom-bar-height')} + env(safe-area-inset-bottom,0px) + ${sp('xl')})`),
  'lx-toolbar': `flex items-center box-border w-full gap-[${sp('sm')}] px-[${sp('md')}] min-h-[${v('header-height')}]`,
  'lx-toolbar-title': `flex-1 min-w-0 overflow-hidden text-ellipsis whitespace-nowrap ${fs('lg')} ${p('font-weight', v('font-weight-strong'))}`,
  'lx-drawer-fixed-left': `sticky box-border shrink-0 overflow-y-auto ${p('grid-column', '1')} ${p('grid-row', '2')} top-[${v('header-height')}] h-[calc(100vh-${v('header-height')}-var(--lx-layout-footer-space,0px))] ${p('background', v('drawer-bg'))}`,
  'lx-drawer-fixed-right': `sticky box-border shrink-0 overflow-y-auto ${p('grid-column', '3')} ${p('grid-row', '2')} top-[${v('header-height')}] h-[calc(100vh-${v('header-height')}-var(--lx-layout-footer-space,0px))] ${p('background', v('drawer-bg'))}`,
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

  // ── Table ─────────────────────────────────────────────────────────────────
  // O quadro (título, progresso, tabela, estado vazio, rodapé); o estilo das células é do `lx-mt`.
  'lx-table': [
    'relative flex flex-col overflow-hidden box-border',
    `rounded-[${v('radius-md')}]`,
    p('background', v('card-bg')),
    color('on-surface'),
    fs('sm'),
  ].join(' '),
  'lx-table-title': `px-[${sp('md')}] py-[${sp('sm')}] ${p('font-size', v('table-title-size'))} ${p('font-weight', v('table-title-weight'))}`,
  'lx-table-progress': `relative shrink-0 overflow-hidden h-[${v('table-progress-height')}]`,
  'lx-table-align-left': 'text-left',
  'lx-table-align-center': 'text-center',
  'lx-table-align-right': 'text-right',
  'lx-table-sort': [
    'inline-flex items-center border-0 bg-transparent p-0 cursor-pointer',
    `gap-[${sp('xs')}]`,
    p('font', 'inherit'),
    p('color', 'inherit'),
    p('text-transform', 'inherit'),
    'focus-visible:outline-2 focus-visible:outline-solid focus-visible:outline-offset-2',
    `focus-visible:[outline-color:${v('color-primary')}]`,
  ].join(' '),
  'lx-table-sort-icon': `inline-block shrink-0 transition-[transform,opacity] duration-[${v('duration-fast')}]`,
  'lx-table-sort-icon-idle': p('opacity', v('table-sort-idle-opacity')),
  'lx-table-sort-icon-asc': 'opacity-100',
  'lx-table-sort-icon-desc': `opacity-100 ${p('transform', 'rotate(180deg)')}`,
  'lx-table-empty': border('top'),
  'lx-table-loading': `flex items-center justify-center gap-[${sp('sm')}] px-[${sp('md')}] py-[${sp('xl')}] ${color('on-surface-muted')}`,
  'lx-table-footer': [
    'flex flex-wrap items-center justify-end box-border',
    `gap-[${sp('md')}] px-[${sp('md')}] py-[${sp('xs')}]`,
    border('top', 'table-footer-border'),
    p('color', v('table-footer-color')),
  ].join(' '),
  'lx-table-pages': `flex items-center gap-[${sp('xs')}]`,
}
