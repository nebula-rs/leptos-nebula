# Sample hosts

Hostile host pages for the web component mode (`wc`, ADR-0018 in the root `docs/`). Each one is its own pnpm package and Moon project, versioned with `leptos-nebula`. For now they only have **empty slots**: the nebula micro-frontends are mounted once R-51 exists.

| Folder | Moon id | Port | Micro-frontend management | Hostile CSS |
|---|---|---|---|---|
| `federation/` | `host-federation` | 5301 | Module Federation (`@module-federation/vite`) | PicoCSS + `src/hostile.css` |
| `vanilla/` | `host-vanilla` | 5302 | Manual: two slots (cards) mounted/unmounted independently, in parallel | NES.css + `src/hostile.css` |

Both `hostile.css` files try to leak into the slots on purpose: global `*` rules, inherited properties on `body`, `--lx-*` tokens on `:root`, `!important`, element selectors for `button`/`input`/`select`/`dialog`.
