// Client side of the dev hook: merges the dev mounts (env/HTTP via the virtual module, plus the
// `?nebula-mount=/path@url` query) into the host's routes. Shared by both sample hosts.
import devMounts from 'virtual:nebula-dev-mounts'

const QUERY = 'nebula-mount'

function fromQuery() {
  return new URLSearchParams(location.search).getAll(QUERY).flatMap((v) => {
    const i = v.indexOf('@')
    return i > 0 ? [{ path: '/' + v.slice(0, i).replace(/^\/+|\/+$/g, ''), url: v.slice(i + 1) }] : []
  })
}

/** `routes`: `[{ path, title, url | null }]` → same shape, dev mounts applied (they win; new paths are added). */
export function withDevMounts(routes) {
  const out = routes.map((r) => ({ ...r }))
  for (const m of [...devMounts, ...fromQuery()]) {
    const hit = out.find((r) => r.path === m.path)
    if (hit) Object.assign(hit, { url: m.url, dev: true })
    else out.push({ path: m.path, title: `dev ${m.path}`, url: m.url, dev: true })
  }
  return out
}

/**
 * Route that owns `pathname`: exact match, else the longest path prefix (`/docs` owns
 * `/docs/x`; `/` owns everything left). Lets a mounted app keep its own router below its path.
 * Without a match, the first route.
 */
export function matchRoute(routes, pathname) {
  const owns = (p) => p === pathname || p === '/' || pathname.startsWith(p + '/')
  return routes.filter((r) => owns(r.path)).sort((a, b) => b.path.length - a.path.length)[0] ?? routes[0]
}
