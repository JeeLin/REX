import { describe, it, expect, vi } from 'vitest'
import { mount, type VueWrapper } from '@vue/test-utils'
import ShortcutPanel from '../ShortcutPanel.vue'
import { SHORTCUT_PANEL_GROUPS } from '../shortcuts'
import zhLocale from '@/i18n/locales/zh.json'
import enLocale from '@/i18n/locales/en.json'

type LocaleDict = Record<string, unknown>

/** Resolve a dotted i18n key against a locale file; undefined when missing. */
function resolveKey(dict: unknown, key: string): string | undefined {
  let node: unknown = dict
  for (const part of key.split('.')) {
    if (node === null || typeof node !== 'object') return undefined
    node = (node as LocaleDict)[part]
  }
  return typeof node === 'string' ? node : undefined
}

function tr(locale: 'zh' | 'en', key: string): string {
  return resolveKey(locale === 'zh' ? zhLocale : enLocale, key) ?? key
}

const i18nState = vi.hoisted(() => ({ locale: 'zh' }))

vi.mock('vue-i18n', async () => {
  // Resolve against the real locale files so the rendered copy is asserted
  // against zh/en json instead of raw key names.
  const [{ default: zh }, { default: en }] = await Promise.all([
    import('@/i18n/locales/zh.json'),
    import('@/i18n/locales/en.json'),
  ])
  const locales: Record<string, unknown> = { zh, en }
  return {
    useI18n: () => ({
      t: (key: string, fallback?: string) =>
        resolveKey(locales[i18nState.locale], key) ?? fallback ?? key,
      // The panel reads `locale.value`, so expose a ref-shaped getter rather
      // than the plain state object.
      locale: {
        get value(): string {
          return i18nState.locale
        },
      },
    }),
  }
})

// ── Implementation-side key inventory ─────────────────────────────────────
// Every key the app actually binds, transcribed from the handler sources
// (file + site noted per entry). The panel may only claim keys found here or
// in PASSTHROUGH_KEYS — that is the claim ↔ handler cross-check.
const IMPLEMENTED_KEYS: Record<string, string> = {
  // workspace: WorkspacePage.vue useKeyboardShortcuts + AppLayout.vue
  'Ctrl+K': 'AppLayout.vue handleGlobalKeydown — command palette',
  'Ctrl+Shift+N': 'WorkspacePage.vue — router.push("/workspace")',
  'Alt+T': 'WorkspacePage.vue — new SSH tab',
  'Alt+W': 'WorkspacePage.vue — close current tab',
  'Ctrl+Shift+→': 'WorkspacePage.vue — cycleTab(1)',
  'Ctrl+Shift+←': 'WorkspacePage.vue — cycleTab(-1)',
  'Alt+Shift+T': 'WorkspacePage.vue — reopenClosedTab',
  'Alt+1~9': 'WorkspacePage.vue — jumpToTab(1..9)',
  'Ctrl+\\': 'WorkspacePage.vue — splitHorizontal',
  'Ctrl+Shift+\\': 'WorkspacePage.vue handleSplitKeydown — splitVertical',
  'Ctrl+Alt+1': 'WorkspacePage.vue — layout preset: single',
  'Ctrl+Alt+2': 'WorkspacePage.vue — layout preset: left-right',
  'Ctrl+Alt+3': 'WorkspacePage.vue — layout preset: top-bottom',
  'Ctrl+Alt+4': 'WorkspacePage.vue — layout preset: four grid',
  'Ctrl+Alt+5': 'WorkspacePage.vue — layout preset: main + side',
  'F11': 'AppLayout.vue handleGlobalKeydown — fullscreen toggle',
  'F1': 'WorkspacePage.vue — shortcut panel toggle',
  // ssh terminal: WorkspaceTerminal.vue attachCustomKeyEventHandler
  'Ctrl+Shift+C': 'WorkspaceTerminal.vue — force copy selection',
  'Ctrl+Shift+V': 'WorkspaceTerminal.vue — xterm bracketed paste',
  'Ctrl+F': 'WorkspaceTerminal.vue / SqlEditor searchKeymap — find',
  'Ctrl+Shift+F':
    'WorkspaceTerminal.vue toggle SFTP / SqlPage global search / SqlEditor format',
  // sql console: SqlPage.vue handleKeydown + SqlEditor.vue keymap
  'Ctrl+Enter': 'SqlEditor.vue keymap — execute',
  'Ctrl+S': 'SqlEditor.vue keymap — save query',
  'Ctrl+Shift+R': 'SqlEditor.vue keymap — find & replace',
  'Ctrl+Shift+Q': 'SqlPage.vue — global query',
  'Ctrl+Shift+A': 'SqlPage.vue — AI assistant',
  // files: FilesPage.vue keydown
  'F2': 'FilesPage.vue — rename',
  'F7': 'FilesPage.vue — new folder',
  'F8 / Delete': 'FilesPage.vue — delete selected',
  'Ctrl+R': 'FilesPage.vue — refresh list',
}

// Keys with no app-level handler *by design*: the keystroke is forwarded to
// the remote shell (PRODUCT.md §5: 清屏（透传至 shell 执行）).
const PASSTHROUGH_KEYS: Record<string, string> = {
  'Ctrl+L': 'terminal clear runs in the remote shell (no app handler)',
}

/** Claims that no handler (nor pass-through path) backs. */
function unboundClaims(claims: string[]): string[] {
  return claims.filter((k) => !(k in IMPLEMENTED_KEYS) && !(k in PASSTHROUGH_KEYS))
}

// Browser-reserved / rebound keys the panel must never advertise.
const FORBIDDEN_KEYS = [
  'Ctrl+N',
  'Ctrl+T',
  'Ctrl+W',
  'Ctrl+Tab',
  'Ctrl+Shift+Tab',
  'Ctrl+Shift+T',
  'Tab',
  'F4',
  'F5',
  'F6',
  'Alt+1',
  'Alt+2',
  'Alt+3',
  'Alt+4',
  'Alt+5',
]

const SCOPE_KEYS = [
  'shortcuts.scopeEditorFocused',
  'shortcuts.scopeEditorBlurred',
  'shortcuts.scopeTerminalFocused',
]

function mountPanel(locale: 'zh' | 'en' = 'zh'): VueWrapper {
  i18nState.locale = locale
  return mount(ShortcutPanel, {
    props: { show: true },
    global: { stubs: { teleport: true } },
  })
}

function renderedKeys(wrapper: VueWrapper): string[] {
  return wrapper.findAll('.sp-keys').map((k) => k.text())
}

function dataKeys(): string[] {
  return SHORTCUT_PANEL_GROUPS.flatMap((g) => g.shortcuts.map((s) => s.keys))
}

describe('ShortcutPanel claims ↔ implementation bindings', () => {
  it('claims only keys the app actually binds (or passes through)', () => {
    expect(unboundClaims(dataKeys())).toEqual([])
  })

  it('rejects claims nobody handles (negative controls)', () => {
    expect(unboundClaims(['Ctrl+T'])).toEqual(['Ctrl+T'])
    expect(unboundClaims([...dataKeys(), 'Ctrl+Tab'])).toEqual(['Ctrl+Tab'])
    expect(unboundClaims(['Ctrl+Shift+X'])).toEqual(['Ctrl+Shift+X'])
  })

  it('never claims removed or rebinding keys', () => {
    const keys = dataKeys()
    for (const key of FORBIDDEN_KEYS) {
      expect(keys, `forbidden key ${key} must not be claimed`).not.toContain(key)
    }
  })

  it('keeps Ctrl+L as a shell pass-through, not an app handler', () => {
    expect(dataKeys()).toContain('Ctrl+L')
    expect(PASSTHROUGH_KEYS['Ctrl+L']).toBeDefined()
    expect(IMPLEMENTED_KEYS['Ctrl+L']).toBeUndefined()
  })

  it('inventory still covers the finalized v0.90.0 workspace keys', () => {
    // Guards against "fixing" the cross-check by deleting inventory entries.
    const finalized = [
      'Alt+T',
      'Alt+W',
      'Ctrl+Shift+→',
      'Ctrl+Shift+←',
      'Alt+Shift+T',
      'Alt+1~9',
      'Ctrl+Alt+1',
      'Ctrl+Alt+5',
      'Ctrl+Shift+N',
      'F11',
      'F1',
    ]
    for (const key of finalized) {
      expect(IMPLEMENTED_KEYS[key], `${key} missing from inventory`).toBeDefined()
    }
  })
})

describe('ShortcutPanel rendering', () => {
  it('renders the data module table', () => {
    expect(renderedKeys(mountPanel())).toEqual(dataKeys())
  })

  it('resolves every title and desc key in zh and en', () => {
    for (const group of SHORTCUT_PANEL_GROUPS) {
      expect(resolveKey(zhLocale, group.title), group.title).toBeTruthy()
      expect(resolveKey(enLocale, group.title), group.title).toBeTruthy()
      for (const s of group.shortcuts) {
        expect(s.desc, `${s.keys} desc`).toMatch(/\./)
        expect(resolveKey(zhLocale, s.desc), s.desc).toBeTruthy()
        expect(resolveKey(enLocale, s.desc), s.desc).toBeTruthy()
      }
    }
  })

  it('keeps scope qualifiers in the locale files', () => {
    for (const key of SCOPE_KEYS) {
      expect(resolveKey(zhLocale, key), key).toBeTruthy()
      expect(resolveKey(enLocale, key), key).toBeTruthy()
    }
  })
})

describe('ShortcutPanel Ctrl+Shift+F scope copy', () => {
  it('labels the three contexts distinctly in zh', () => {
    const rows = mountPanel('zh').findAll('.sp-row')
    const scoped = rows
      .filter((r) => r.find('.sp-keys').text() === 'Ctrl+Shift+F')
      .map((r) => r.find('.sp-desc').text())

    expect(scoped).toHaveLength(3)
    expect(scoped).toContain(
      `${tr('zh', 'shortcuts.formatSQL')}（${tr('zh', 'shortcuts.scopeEditorFocused')}）`,
    )
    expect(scoped).toContain(
      `${tr('zh', 'shortcuts.globalSearch')}（${tr('zh', 'shortcuts.scopeEditorBlurred')}）`,
    )
    expect(scoped).toContain(
      `${tr('zh', 'terminal.openSftp')}（${tr('zh', 'shortcuts.scopeTerminalFocused')}）`,
    )
  })

  it('labels the three contexts distinctly in en', () => {
    const rows = mountPanel('en').findAll('.sp-row')
    const scoped = rows
      .filter((r) => r.find('.sp-keys').text() === 'Ctrl+Shift+F')
      .map((r) => r.find('.sp-desc').text())

    expect(scoped).toHaveLength(3)
    expect(scoped).toContain(
      `${tr('en', 'shortcuts.formatSQL')} (${tr('en', 'shortcuts.scopeEditorFocused')})`,
    )
    expect(scoped).toContain(
      `${tr('en', 'shortcuts.globalSearch')} (${tr('en', 'shortcuts.scopeEditorBlurred')})`,
    )
    expect(scoped).toContain(
      `${tr('en', 'terminal.openSftp')} (${tr('en', 'shortcuts.scopeTerminalFocused')})`,
    )
  })

  it('assigns a unique scope to each Ctrl+Shift+F entry', () => {
    const entries = SHORTCUT_PANEL_GROUPS.flatMap((g) => g.shortcuts).filter(
      (s) => s.keys === 'Ctrl+Shift+F',
    )
    expect(entries.map((e) => e.scope).sort()).toEqual([
      'editorBlurred',
      'editorFocused',
      'terminalFocused',
    ])
  })
})
