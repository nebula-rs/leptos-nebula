import 'nes.css/css/nes.min.css'
import './hostile.css'
import { withDevMounts } from '@leptos-nebula/dev-mounts/client'
import { slots as own, mount, unmount } from './registry.js'

// Dev mounts: `/a`/`/b` target one slot; `/` (the `docs:dev:wc` default) fills every slot.
const merged = withDevMounts(own)
const everywhere = merged.find((r) => r.path === '/')?.url
const slots = own.map((s) => {
  const dev = merged.find((r) => r.path === s.path)
  return { ...s, url: dev?.url ?? everywhere ?? s.url, dev: !!dev?.dev || !!everywhere }
})

const list = document.getElementById('slots')

for (const slot of slots) {
  const card = document.createElement('section')
  card.className = 'nes-container with-title slot-card'
  card.innerHTML = `
    <p class="title">${slot.title}${slot.dev ? ' (dev)' : ''}</p>
    <div class="nes-field slot-url">
      <label for="url-${slot.path.slice(1)}">Módulo</label>
      <input id="url-${slot.path.slice(1)}" class="nes-input" placeholder="http://localhost:8090/mf.js" />
    </div>
    <button type="button" class="nes-btn is-success" data-act="mount">Montar</button>
    <button type="button" class="nes-btn is-error" data-act="unmount">Desmontar</button>
    <span class="nes-text is-disabled" data-status>desmontado</span>
    <div class="slot" data-slot="${slot.path}"></div>`
  list.append(card)
  const input = card.querySelector('input')
  const target = card.querySelector('.slot')
  const status = card.querySelector('[data-status]')
  input.value = slot.url ?? ''
  card.querySelector('[data-act=mount]').onclick = async () => {
    const url = input.value.trim()
    if (!url) return
    target.textContent = ''
    status.textContent = 'montando…'
    try {
      await mount(slot, url, target)
      status.textContent = 'montado'
    } catch (e) {
      status.textContent = 'falha'
      target.textContent = String(e)
    }
  }
  card.querySelector('[data-act=unmount]').onclick = () => {
    if (unmount(slot)) {
      target.textContent = ''
      status.textContent = 'desmontado'
    }
  }
}

