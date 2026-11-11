// Dev hook shared by the sample hosts: lets a leptos-nebula app under development tell a host
// "mount me (this ES module) at path Y". Dev server only (`apply: 'serve'`); the build ignores it.
//
// Three ways to register a mount `{ path, url }` (`url` = ES module exposing `mount(el) -> unmount()`):
//   1. env at startup:  NEBULA_DEV_MOUNTS="/y=http://localhost:8080/mf.js,/z=http://…"
//   2. HTTP, at runtime: POST /__nebula/mounts  {"path":"/y","url":"…"}   (DELETE with {"path"} removes)
//                        GET  /__nebula/mounts  lists them. The page reloads on every change.
//   3. query string (one tab only): ?nebula-mount=/y@http://localhost:8080/mf.js
// Dev mounts take precedence over the host's own slot for the same path.

const VIRTUAL = 'virtual:nebula-dev-mounts'
const RESOLVED = '\0' + VIRTUAL
export const ENDPOINT = '/__nebula/mounts'

export function parseEnv(value) {
  const mounts = new Map()
  for (const pair of (value ?? '').split(',').map((s) => s.trim()).filter(Boolean)) {
    const i = pair.indexOf('=')
    if (i > 0) mounts.set(normalize(pair.slice(0, i)), pair.slice(i + 1))
  }
  return mounts
}

function normalize(path) {
  return '/' + path.replace(/^\/+|\/+$/g, '')
}

function readBody(req) {
  return new Promise((resolve, reject) => {
    let data = ''
    req.on('data', (c) => (data += c))
    req.on('end', () => {
      try { resolve(data ? JSON.parse(data) : {}) } catch (e) { reject(e) }
    })
  })
}

export function nebulaDevMounts() {
  const mounts = parseEnv(process.env.NEBULA_DEV_MOUNTS)
  let server
  const list = () => [...mounts].map(([path, url]) => ({ path, url }))
  const changed = () => {
    const mod = server.moduleGraph.getModuleById(RESOLVED)
    if (mod) server.moduleGraph.invalidateModule(mod)
    server.ws.send({ type: 'full-reload' })
  }
  return {
    name: 'nebula-dev-mounts',
    apply: 'serve',
    resolveId: (id) => (id === VIRTUAL ? RESOLVED : null),
    load: (id) => (id === RESOLVED ? `export default ${JSON.stringify(server ? list() : [])}` : null),
    configureServer(s) {
      server = s
      s.middlewares.use(ENDPOINT, async (req, res) => {
        res.setHeader('content-type', 'application/json')
        res.setHeader('access-control-allow-origin', '*')
        res.setHeader('access-control-allow-methods', 'GET, POST, DELETE')
        res.setHeader('access-control-allow-headers', 'content-type')
        try {
          if (req.method === 'POST' || req.method === 'DELETE') {
            const { path, url } = await readBody(req)
            if (!path || (req.method === 'POST' && !url)) {
              res.statusCode = 400
              return res.end(JSON.stringify({ error: 'expected {"path","url"}' }))
            }
            if (req.method === 'POST') mounts.set(normalize(path), url)
            else mounts.delete(normalize(path))
            changed()
          }
          res.end(JSON.stringify(list()))
        } catch (e) {
          res.statusCode = 400
          res.end(JSON.stringify({ error: String(e) }))
        }
      })
      if (mounts.size) s.config.logger.info(`  nebula dev mounts: ${list().map((m) => `${m.path} → ${m.url}`).join(', ')}`)
    },
  }
}

// Build: an empty module, so `import devMounts from 'virtual:nebula-dev-mounts'` always resolves.
export function nebulaDevMountsBuild() {
  return {
    name: 'nebula-dev-mounts-build',
    apply: 'build',
    resolveId: (id) => (id === VIRTUAL ? RESOLVED : null),
    load: (id) => (id === RESOLVED ? 'export default []' : null),
  }
}
