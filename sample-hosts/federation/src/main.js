import '@picocss/pico/css/pico.min.css'
import './hostile.css'
import { matchRoute, withDevMounts } from '@leptos-nebula/dev-mounts/client'
import { routes as own } from './slots.js'

const routes = withDevMounts(own)
const nav = document.getElementById('routes')
const outlet = document.getElementById('outlet')
let current = null
let unmount = null

for (const r of routes) {
  const li = document.createElement('li')
  li.innerHTML = `<a href="${r.path}">${r.title}${r.dev ? ' (dev)' : ''}</a>`
  nav.append(li)
}

document.addEventListener('click', (e) => {
  // Only the host's own nav: links inside a mounted app belong to that app's router.
  const a = e.target.closest?.('#routes a[href^="/"]')
  if (!a) return
  e.preventDefault()
  history.pushState(null, '', a.getAttribute('href') + location.search)
  render()
})
// Back/forward inside a mounted app is that app's business: re-render only when the owner changes.
addEventListener('popstate', () => {
  if (matchRoute(routes, location.pathname) !== current) render()
})

async function render() {
  unmount?.()
  unmount = null
  const route = matchRoute(routes, location.pathname)
  current = route
  outlet.innerHTML = `<header>${route.title}</header><div class="slot" data-path="${route.path}"></div>`
  const target = outlet.querySelector('.slot')
  const spec = route.url ?? route.remote
  if (!spec) {
    target.textContent = 'slot vazio'
    return
  }
  // Contract of a micro-frontend module: `mount(el) -> unmount()`.
  try {
    const m = await import(/* @vite-ignore */ spec)
    unmount = m.mount(target) ?? null
  } catch (e) {
    target.textContent = `falha ao carregar ${spec}: ${e}`
  }
}

render()
