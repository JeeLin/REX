# REX 1.0 Prototype — Design & Development Guide

> **Stage: 1.0.** These prototypes are the **1.0** generation. Wherever a version
> string appears in the UI it must read **1.0.x** — never 2.0. Do not
> reintroduce 2.0 version labels. (The containing folder is named `rex-1.0/` —
> the 1.0 prototype set.)

These are static HTML prototypes for **REX Hub** — a single-user, self-hosted,
dark-first remote-resource console (SSH / SQL / Redis / SFTP / S3 / SIP). This
document is the **spec to develop against**: it records the OpenDesign direction,
the token/component library every page must reuse, the shared interaction runtime,
and step-by-step recipes for extending the prototype. Edit HTML/CSS/JS directly —
there is no build step.

---

## 1. Tech & how to run

- **Pure static HTML + CSS + vanilla JS.** No framework, no bundler, no server.
- Open any `NN-*.html` directly in a browser. Sidebar links use plain relative
  paths (`href="02-workspace.html"`), so open from inside `prototypes/rex-1.0/`.
- Shared code in `assets/`, included by every page:
  - `assets/tokens.css` — unified `:root` design tokens (colors, typography, geometry). **Single source of truth** — edit tokens here ONLY.
  - `assets/shell.css` — shared layout / chrome CSS (resets, app grid, sidebar, nav, buttons, badges, status dots, topbar, content area). Depends on tokens.css.
  - `assets/common.css` — overlay components (modal / confirm / toast / context-menu / empty-state / command-palette / lock screen).
  - `assets/common.js` — global `REX` runtime (see §6).
- Optional external tool: **OpenDesign** for the anti-slop pass
  (`open-design lint NN-*.html`). Loopback daemon, no token:
  `open-design --no-open --host 127.0.0.1 --port 7456`.

## 2. File structure

```
prototypes/rex-1.0/
├── DESIGN.md            ← this file
├── 00-login.html        Sign-in
├── 01-dashboard.html    Overview / landing after login
├── 02-workspace.html    ★ Core work canvas (most logic; model for new canvas work)
├── 07-environments.html Environments (List + Topology tabs)
├── 08-agents.html       Agents
├── 09-settings.html     Settings
├── 10-audit.html        Audit log
└── assets/
    ├── tokens.css      unified :root design tokens
    ├── shell.css       shared layout / chrome CSS
    ├── common.css      overlay components (modal / toast / context-menu)
    └── common.js       REX.* runtime
Numbered prefixes encode the original page order; keep them stable so sidebar
links don't break.

## 3. OpenDesign direction: `tech-utility`

`od tools directions --id tech-utility` → *Datadog / GitHub / Cloudflare / Sentry*
— data-dense, monospace-friendly, grid-first. Closest built-in to REX's baseline
(PRODUCT.md §0). Other built-ins are off-brand for an ops console.

**Posture rules (apply to all new UI):**
- **Tabular numerics everywhere** — `font-variant-numeric: tabular-nums` on `body`.
- **Mono for code / IDs / hashes** — `--font-mono` for usernames, hosts,
  connection strings, token IDs, metric labels.
- **Dense tables, hairline borders, no zebra** — `var(--border)` + last-row border
  removal.
- **Inline status pills** — tinted backgrounds (`rgba(... ,.14)`), not loud fills.
- **No hero images / oversized headlines / marketing copy** — show the product.

## 4. Anti-slop lint (`od lint`)

- **P0 `ai-default-indigo`** (`#8B5CF6` / `var(--purple)`) — **KEPT.** PRODUCT.md §2.3
  specifies it as the deliberate brand color for **PostgreSQL (`pg`) and SFTP**
  only. Exists solely as the `--purple` token, referenced via `var(--purple)`.
  Linter false-positive for this product.
- **P1 `all-caps-no-tracking`** — uppercase mono labels need `letter-spacing ≥
  0.08em`.
- **P1 `raw-hex`** — colors outside `:root` must be promoted to tokens. No new raw
  hex in markup.

## 5. Token system (`assets/tokens.css`)

All HTML files include `assets/tokens.css` via `<link>`. **Edit a token in `tokens.css` only** — changes propagate to every page automatically.
| Token | Value | Role |
|-------|-------|------|
| `--bg-app` | `#0E1116` | app background |
| `--bg-sidebar` | `#0A0D12` | left sidebar |
| `--bg-surface` | `#161B22` | cards / panels |
| `--bg-elevated` | `#1C2128` | headers / sticky rows |
| `--bg-hover` | `#21262D` | hover surface |
| `--bg-terminal` | `#0D1117` | terminal / code blocks |
| `--border` | `#232A33` | hairline borders |
| `--border-strong` | `#30363D` | input / button borders |
| `--text` | `#E6EDF3` | primary text |
| `--text-muted` | `#8B949E` | secondary text |
| `--text-dim` | `#6E7681` | tertiary / captions |
| `--brand` | `#E8912D` | **only** accent (orange) |
| `--brand-strong` | `#F2A33D` | accent hover |
| `--brand-soft` | `rgba(232,145,45,.14)` | accent tint |
| `--purple` | `#8B5CF6` | **PostgreSQL / SFTP only** (intentional) |
| `--success` | `#3FB950` | online / ok |
| `--danger` | `#F85149` | error / fail / Redis |
| `--info` | `#58A6FF` | MySQL / direct-connect |
| `--teal` | `#2DD4BF` | **SIP** protocol |
| `--warning` | `#D29922` | SQLite / degraded |
| `--font-mono` | JetBrains Mono stack | code / IDs / metrics |
| `--font-sans` | Inter stack | UI text |
| `--r / --r-lg` | `8px / 12px` | radii |
| `--sidebar-w` | `256px` | sidebar width |

**Protocol color classes** (sidebar, dashboards, topology): `.p-ssh`=success,
`.p-sftp`=purple, `.p-mysql`=info, `.p-pg`=purple, `.p-redis`=danger,
`.p-sqlite`=warning, `.p-s3`=brand, `.p-sip`=teal.

## 6. Component inventory (reuse, don't reinvent)

| Class / element | Purpose | Notes |
|-----------------|---------|-------|
| `.app` | `grid-template-columns: var(--sidebar-w) 1fr` | page shell |
| `.sidebar` / `.side-head` / `.brand` | left rail | identical across pages |
| `.search .box` | global search box (Cmd/Ctrl K) | decorative in prototype |
| `.nav a` (+ `.active`) | primary nav | Workspace/Dashboard/Environments/Audit/Settings |
| `.side-section` + `.tree-body .grp/.res` | connection tree | `.res[data-kind]` is clickable → opens resource |
| `.btn` / `.btn-primary` / `.btn-ghost` | buttons | |
| `.tbtn` / `.tbtn.primary` | toolbar buttons | |
| `.badge` + `.badge.green/.blue/.gray/.red/.brand` | status pills | |
| `.st` + `.st.on/.off/.warn/.pulse` | 8px status dot | |
| `.stat` (`.lab`+`.num`+`.ico`) | KPI card | |
| `.panel` / `.panel-head` | card with header | |
| `.table.t` | data table | hairline, no zebra |
| `.grid` + `.env` card | responsive card grid | |
| `.tabbar` / `.tab` | resource/workgroup tabs (workspace) | |
| `.split.row/.col` + `.leaf` + `.divider` | recursive free split (workspace) | see §8 |
| `REX.*` (common.js) | modal / toast / context menu / store | see §7 |

## 7. Interaction runtime (`assets/common.js` → global `REX`)

Every page includes `common.js`. Use `REX.*` for all overlays — never hand-roll
`<div>` popups.

| API | Signature | Behavior |
|-----|-----------|----------|
| `REX.toast(msg, type)` | `type ∈ ok\|err\|info` | bottom-right, auto-dismiss ~2.6s |
| `REX.modal(opts)` | `{title, body, okText, onOk(values), onCancel, width}` | form dialog; `body` may contain `<input data-field="k">`; `onOk` receives a `{k: value}` map; **return `false` to block close** (validation) |
| `REX.confirm(opts, onYes)` | `{title, message, okText, danger}` | confirm-then-act |
| `REX.contextMenu(items, x, y)` | `items: {label, onClick} \| {sep:true} \| {label, danger, onClick}` | right-click menu |
| `REX.store(key, initial)` | → `{get, set, clear}` | localStorage wrapper |

**Conventions for new interactions:**
- New / Edit / Delete → `REX.modal` / `REX.confirm`. No inline forms.
- The **New-resource dialog is type-aware**: picking a kind swaps in that kind's
  connection fields (`RES_CREATE` map in `02-workspace.html`); creation saves the
  params, injects the tree item, and opens the resource.
- Destructive (delete env, reset token) → **always confirm**.
- Any actionable list row → right-click `REX.contextMenu` mirroring its buttons.
- Demonstrate "data" via `REX.toast(...)` + navigation; persistence optional via
  `REX.store`.

## 8. Workspace model (`02-workspace.html`) — the core

Model new canvas work on this page (agreed **方案 B**):

- **Multi workspace groups** (top tabs `主用 / 排障 / ...`): add / switch / close;
  each group is an independent saved layout, persisted to `localStorage`
  (`rex.workspace.v1`) — on the product this becomes the admin profile.
- **Mosaic canvas**: tree of `.split.row` / `.split.col` holding `.leaf` + `.divider`.
  Any leaf splits left/right or up/down, nested arbitrarily; dividers are
  pointer-drag resizable (15%–85%).
- **Each leaf is resource-agnostic** — holds *any* kind (SSH / SQL / Redis / SFTP
  files / S3 / SIP). "Left SSH, right SQL" is one layout, not separate pages.
  Empty leaves show a **resource picker**; picking a kind loads that body.
- **Open a specific resource**: click a `.res[data-kind]` in the left tree → fills
  the first empty leaf (or adds one); the leaf shows the connection **name**.
  The picker only chooses *type*.
- **Open an environment** (Environments page card/menu) hands the workspace an
  `{env}` intent via `rex.open-intent`: it collapses other sidebar groups,
  expands + scrolls to that environment's group (creating an empty one if the
  env has no resources yet), and opens its first connection.
- **Switch a leaf's resource**: click the leaf **title** (or right-click the panel →
  切换资源) → picker; `▣`/`▭` split; `✕` close (single leaf resets to empty).
- **SIP** kind has an in-connection **account switcher**: a compact `▾` selector in
  the panel bar opens a scrollable, filterable popover — so the top bar stays one line
  tall no matter how many SIP identities exist. Switching account keeps the same
  connection, changes only the registered identity — persisted on the leaf as `acct`.

**Add a resource kind:** extend `RES` with `{label, cls, icon, body}`, add its
`.p-*` color, and add a `.res[data-kind][data-name]` in the sidebar tree. The
leaf engine, split, and persistence need no changes.

## 9. Page skeleton (copy for a new page)

Every page shares this shell. Copy, then fill `#main`:

```html
<!doctype html><html lang="en"><head>
<meta charset="utf-8"/><meta name="viewport" content="width=device-width,initial-scale=1"/>
<title>REX · Page</title>
<link rel="stylesheet" href="assets/tokens.css"/>
<link rel="stylesheet" href="assets/shell.css"/>
<link rel="stylesheet" href="assets/common.css"/>
<style>
  /* page-specific styles go here — tokens and shell are in external files */
</style>
</head>
<link rel="stylesheet" href="assets/common.css"/>
</head>
<body>
<div class="app">
  <aside class="sidebar">
    <div class="side-head"><div class="brand"><span class="glyph">R</span><span class="name">RE<b>X</b></span></div>
      <div class="side-tools"><button class="icon-btn" title="Theme">…</button></div></div>
    <nav class="nav">
      <a href="02-workspace.html">… Workspace</a>
      <a href="01-dashboard.html">… Dashboard</a>
      <a href="07-environments.html">… Environments</a>
      <a href="10-audit.html">… Audit log</a>
      <a href="09-settings.html">… Settings</a>
    </nav>
    <div class="side-foot">
      <button class="btn btn-ghost" style="flex:1">+ New environment</button>
      <a class="icon-btn" href="09-settings.html" title="Settings"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="3"/><path d="M19 12a7 7 0 0 0-.1-1.3l2-1.5-2-3.4-2.3 1a7 7 0 0 0-2.2-1.3L14 2h-4l-.4 2.5a7 7 0 0 0 2.2 1.3l-2.3-1-2 3.4 2 1.5A7 7 0 0 0 5 12c0 .4 0 .9.1 1.3l-2 1.5 2 3.4 2.3-1a7 7 0 0 0 2.2 1.3L10 22h4l.4-2.5a7 7 0 0 0 2.2 1.3l2.3 1 2-3.4-2-1.5c.1-.4.1-.9.1-1.3z"/></svg></a>
    </div>
  </aside>
  <div class="main">
    <div class="topbar"><div class="crumbs">REX <span class="dim">/</span> <b>Page</b></div>
      <div class="spacer"></div>
      <div class="avatar" style="width:30px;height:30px;border-radius:50%;background:linear-gradient(140deg,var(--info),#3a7fd0);display:grid;place-items:center;color:#06121f;font-weight:700;font-size:12px;font-family:var(--font-mono)">AD</div>
    </div>
    <div class="content">
      <h1 class="page-title">Page</h1>
      <p class="page-sub">…</p>
      <!-- page body -->
    </div>
  </div>
</div>
<script src="assets/common.js"></script>
<script>
  // page logic here; use REX.* for overlays
</script>
</body></html>
```

## 10. Recipes

**Add a new page** (`NN-foo.html`): copy §9 skeleton, set title/breadcrumb, build
body with the §6 components, wire it with `REX.*`, link it from every other
page's `.nav`. Add the nav SVG icon set used elsewhere (they're inline; copy one).

**Add a modal** (e.g. create dialog):
```js
REX.modal({
  title:'New thing', width:'460px', okText:'Create',
  body:'<div class="rex-field"><label>Name</label><input data-field="name" placeholder="…"></div>',
  onOk:function(v){ if(!v.name){ REX.toast('Name required','err'); return false; }
                    REX.toast('Created '+v.name,'ok'); }
});
```

**Add a right-click menu** on a row:
```js
row.addEventListener('contextmenu', function(e){
  e.preventDefault();
  REX.contextMenu([
    {label:'Open', onClick:openFn},
    {sep:true},
    {label:'Delete', danger:true, onClick:function(){ REX.confirm({title:'Delete',message:'…',okText:'Delete',danger:true}, delFn); }}
  ], e.clientX, e.clientY);
});
```

**Add a resource kind to the workspace** (§8): extend `RES`, add `.p-*` color,
add a `.res[data-kind][data-name]` in the sidebar tree. The engine handles the
rest.

## 11. Responsive

- `@media(max-width:760px)`: sidebar hidden; workspace splits collapse to a single
  column; workspace group tabs become the mobile switcher (PRODUCT.md §4 — full
  screen per resource). Keep tables horizontally scrollable, not reflowed.

## 12. Lint / re-check

```bash
open-design --no-open --host 127.0.0.1 --port 7456   # loopback, no token
for f in *.html; do echo "== $f =="; open-design lint "$f"; done
```
Expected: **1 P0** (`ai-default-indigo` for `--purple`, intentional) + **0 new**
findings. Fix any new P0/P1 before committing; keep only the documented `--purple`
P0.

## 13. Pre-commit checklist

1. Token edits applied to **`assets/tokens.css` only** (single source of truth).
2. New overlays use `REX.*`, not bespoke divs.
3. Protocol colors via `.p-*` / `var(--*)`, no raw hex in markup.
4. Version strings stay `1.0.x` (never 2.0).
5. `node --check` on every `<script>` and on `assets/common.js`.
6. `open-design lint` shows only the intentional `--purple` P0.
7. New page is linked from all other pages' `.nav` (including Agents).
8. Every page links `tokens.css`, `shell.css`, and `common.css` in `<head>`.
