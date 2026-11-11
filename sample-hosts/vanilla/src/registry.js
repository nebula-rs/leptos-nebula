// Manual micro-frontend registry: two slots side by side in the page (cards), each mounted and
// unmounted on its own, so two apps (or two instances of one) can run in parallel. `url` = ES
// module exposing `mount(el) -> unmount()`; `null` = nothing configured. In dev, the nebula dev
// hook fills the URLs (`../dev-mounts`): path `/a` → slot A, `/b` → slot B, `/` → every slot.
export const slots = [
  { path: '/a', title: 'Slot A', url: null },
  { path: '/b', title: 'Slot B', url: null },
]

const mounted = new Map() // path → dispose

export async function mount(slot, url, el) {
  unmount(slot)
  const m = await import(/* @vite-ignore */ url)
  mounted.set(slot.path, (await m.mount(el)) ?? (() => {}))
}

export function unmount(slot) {
  const dispose = mounted.get(slot.path)
  if (!dispose) return false
  dispose()
  mounted.delete(slot.path)
  return true
}
