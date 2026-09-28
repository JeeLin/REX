import { describe, it, expect, vi } from 'vitest'
import { mount, type VueWrapper } from '@vue/test-utils'
import ShortcutPanel, { SHORTCUT_PANEL_GROUPS } from '../ShortcutPanel.vue'

const i18nState = vi.hoisted(() => ({ locale: 'zh' }))

vi.mock('vue-i18n', () => ({
  // The panel reads `locale.value`, so expose a ref-shaped getter rather than
  // the plain state object.
  useI18n: () => ({
    t: (k: string) => k,
    locale: {
      get value(): string {
        return i18nState.locale
      },
    },
  }),
}))

// Finalized v0.90.0 key table (milestone K1 baseline + K4 adjudication).
// Order matters: it mirrors the panel's group order.
const EXPECTED_KEYS = [
  // workspace
  'Ctrl+K',
  'Ctrl+Shift+N',
  'Alt+T',
  'Alt+W',
  'Ctrl+Shift+→',
  'Ctrl+Shift+←',
  'Alt+Shift+T',
  'Alt+1~9',
  'Ctrl+\\',
  'Ctrl+Shift+\\',
  'Ctrl+Alt+1',
  'Ctrl+Alt+2',
  'Ctrl+Alt+3',
  'Ctrl+Alt+4',
  'Ctrl+Alt+5',
  'F11',
  'F1',
  // ssh terminal
  'Ctrl+Shift+C',
  'Ctrl+Shift+V',
  'Ctrl+F',
  'Ctrl+L',
  'Ctrl+Shift+F',
  // sql console
  'Ctrl+Enter',
  'Ctrl+Shift+F',
  'Ctrl+Shift+F',
  'Ctrl+S',
  'Ctrl+F',
  'Ctrl+Shift+R',
  'Ctrl+Shift+Q',
  'Ctrl+Shift+A',
  // files
  'F2',
  'F7',
  'F8 / Delete',
  'Ctrl+R',
]

// Claims that must be gone from the panel.
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

describe('ShortcutPanel key table', () => {
  it('renders exactly the finalized key table', () => {
    const wrapper = mountPanel()
    expect(renderedKeys(wrapper)).toEqual(EXPECTED_KEYS)
  })

  it('contains every finalized key (positive cases)', () => {
    const rendered = renderedKeys(mountPanel())
    for (const key of EXPECTED_KEYS) {
      expect(rendered, `expected key ${key}`).toContain(key)
    }
  })

  it('never claims removed or rebinding keys (negative cases)', () => {
    const rendered = renderedKeys(mountPanel())
    for (const key of FORBIDDEN_KEYS) {
      expect(rendered, `forbidden key ${key} must not be claimed`).not.toContain(key)
    }
  })

  it('keeps the exported source table in sync with the rendered keys', () => {
    expect(dataKeys()).toEqual(EXPECTED_KEYS)
  })

  it('does not claim F4/F5/F6 at the data level', () => {
    const keys = dataKeys()
    for (const key of ['F4', 'F5', 'F6']) {
      expect(keys).not.toContain(key)
    }
    expect(JSON.stringify(SHORTCUT_PANEL_GROUPS)).not.toMatch(/"F[456]"/)
  })

  it('resolves every desc as an i18n key', () => {
    for (const group of SHORTCUT_PANEL_GROUPS) {
      expect(group.title).toMatch(/\./)
      for (const s of group.shortcuts) {
        expect(s.desc, `${s.keys} desc`).toMatch(/\./)
      }
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
    expect(scoped).toContain('shortcuts.formatSQL（编辑器聚焦时）')
    expect(scoped).toContain('shortcuts.globalSearch（编辑器未聚焦时）')
    expect(scoped).toContain('terminal.openSftp（终端聚焦时）')
  })

  it('labels the three contexts distinctly in en', () => {
    const rows = mountPanel('en').findAll('.sp-row')
    const scoped = rows
      .filter((r) => r.find('.sp-keys').text() === 'Ctrl+Shift+F')
      .map((r) => r.find('.sp-desc').text())

    expect(scoped).toHaveLength(3)
    expect(scoped).toContain('shortcuts.formatSQL (editor focused)')
    expect(scoped).toContain('shortcuts.globalSearch (editor not focused)')
    expect(scoped).toContain('terminal.openSftp (terminal focused)')
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
