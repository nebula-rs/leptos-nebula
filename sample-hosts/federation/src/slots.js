// Routes of this host: each path has one slot. `remote` = '<remote name>/<exposed module>'
// (declared in `vite.config.js`), `null` = empty slot. In dev, the nebula dev hook can override
// or add paths with a plain ES module `url` (`../dev-mounts`).
export const routes = [
  { path: '/a', title: 'Slot A', remote: null },
  { path: '/b', title: 'Slot B', remote: null },
]
