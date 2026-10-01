import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount, type VueWrapper } from '@vue/test-utils'
import { computed, nextTick, ref } from 'vue'
import SqlPage from '../SqlPage.vue'
import SqlEditor from '../SqlEditor.vue'
import { PANE_CTX, type PaneCtx } from '@/features/workspace/paneContext'

vi.mock('vue-i18n', () => ({
  // Mirrors vue-i18n's `t(key, defaultMsg)` signature.
  useI18n: () => ({
    t: (k: string, defaultMsg?: string) => defaultMsg ?? k,
    locale: { value: 'zh' },
  }),
}))

vi.mock('@/api/sql', () => ({
  connect: vi.fn(async () => 'session-1'),
  disconnect: vi.fn(async () => undefined),
  getDdl: vi.fn(async () => ({ ddl: '' })),
  executeQuery: vi.fn(async () => ({ rows: [] })),
  getColumns: vi.fn(async () => []),
  getIndexes: vi.fn(async () => []),
  getForeignKeys: vi.fn(async () => []),
  upsertSavedQuery: vi.fn(async () => undefined),
}))

vi.mock('../SqlNavTree.vue', () => ({
  default: { name: 'SqlNavTree', template: '<div class="sql-nav-stub" />' },
}))

// Stand-in for the real modal so the test can read its `visible` prop.
vi.mock('../GlobalSearchModal.vue', () => ({
  default: {
    name: 'GlobalSearchModal',
    props: { visible: { type: Boolean, default: false } },
    template: '<div class="gs-stub" :data-visible="String(visible)" />',
  },
}))

// Visible-prop stand-ins for the other shortcut targets.
const globalQueryStub = {
  props: { visible: { type: Boolean, default: false } },
  template: '<div class="gq-stub" :data-visible="String(visible)" />',
}

const aiAssistantStub = {
  props: { visible: { type: Boolean, default: false } },
  template: '<div class="ai-stub" :data-visible="String(visible)" />',
}

// SqlEditor stays real: the scoping assertions walk its actual tree.
const pageStubs = {
  SqlResultGrid: true,
  TableDesigner: true,
  ExportWizard: true,
  GlobalQueryModal: globalQueryStub,
  AiAssistantDrawer: aiAssistantStub,
  ImportWizard: true,
  SqlFormView: true,
  SavedQueryList: true,
  DataCompare: true,
  Modal: true,
  Input: true,
  Button: true,
}

function press(target: EventTarget, key: string): KeyboardEvent {
  const event = new KeyboardEvent('keydown', {
    key,
    ctrlKey: true,
    shiftKey: true,
    cancelable: true,
    bubbles: true,
  })
  target.dispatchEvent(event)
  return event
}

async function mountSqlPage(): Promise<VueWrapper> {
  return mount(SqlPage, {
    // Shortcuts are bound on `document`, so the page has to be in the tree.
    attachTo: document.body,
    global: { stubs: pageStubs },
  })
}

// Opens a query tab through the page's own toolbar, which is what hosts the
// editor, and hands back the editor root the shortcut scoping checks for.
async function openQueryTab(wrapper: VueWrapper): Promise<HTMLElement> {
  const newTab = wrapper.findAll('button').find((b) => b.attributes('title') === 'sql.newQuery')
  expect(newTab, 'new-query button').toBeDefined()
  await newTab!.trigger('click')
  await nextTick()
  return wrapper.findComponent(SqlEditor).element
}

let wrapper: VueWrapper | null = null
const extraWrappers: VueWrapper[] = []

beforeEach(async () => {
  wrapper = await mountSqlPage()
})

afterEach(() => {
  wrapper?.unmount()
  wrapper = null
  for (const w of extraWrappers) w.unmount()
  extraWrappers.length = 0
  document.body.innerHTML = ''
})

function globalSearchVisible(): boolean {
  return wrapper!.find('.gs-stub').attributes('data-visible') === 'true'
}

function globalQueryVisible(): boolean {
  return wrapper!.find('.gq-stub').attributes('data-visible') === 'true'
}

function aiAssistantVisible(): boolean {
  return wrapper!.find('.ai-stub').attributes('data-visible') === 'true'
}

// Ctrl+Shift+F is context-scoped: SQL editor focused = format (handled inside
// the editor), split terminal focused = terminal's own toggle, SQL page level
// = global search.
describe('SqlPage Ctrl+Shift+F scoping', () => {
  let editorRoot: HTMLElement
  let cmContent: HTMLElement

  beforeEach(async () => {
    editorRoot = await openQueryTab(wrapper!)
    // CodeMirror builds its DOM asynchronously after the editor is created.
    await new Promise((resolve) => setTimeout(resolve, 0))
    const content = editorRoot.querySelector('.cm-content')
    expect(content, 'cm-content').toBeTruthy()
    cmContent = content as HTMLElement
  })

  it('yields to the SQL editor instead of opening global search', async () => {
    // Keydown on the editor root itself.
    const fromRoot = press(editorRoot, 'F')
    await nextTick()
    expect(fromRoot.defaultPrevented).toBe(false)
    expect(globalSearchVisible()).toBe(false)

    // Keydown from a descendant of the editor bubbles the same way. The probe
    // lives outside CodeMirror's own DOM, which owns Ctrl+Shift+F for format.
    const probe = document.createElement('span')
    editorRoot.appendChild(probe)
    const fromInner = press(probe, 'F')
    await nextTick()
    expect(fromInner.defaultPrevented).toBe(false)
    expect(globalSearchVisible()).toBe(false)
    probe.remove()

    // Keydown on CodeMirror's own contenteditable surface: it is still inside
    // `.sql-editor`, so the page must not hijack the editor's shortcut.
    const fromCm = press(cmContent, 'F')
    await nextTick()
    expect(fromCm.defaultPrevented).toBe(false)
    expect(globalSearchVisible()).toBe(false)
  })

  it('yields to the split terminal instead of opening global search', async () => {
    // xterm.js renders its container with the `xterm` root class; while the
    // terminal is focused it owns Ctrl+Shift+F (toggle SFTP).
    const terminal = document.createElement('div')
    terminal.className = 'xterm'
    const inner = document.createElement('span')
    terminal.appendChild(inner)
    document.body.appendChild(terminal)

    const event = press(inner, 'F')
    await nextTick()
    expect(event.defaultPrevented).toBe(false)
    expect(globalSearchVisible()).toBe(false)

    terminal.remove()
  })

  it('opens global search at SQL page level (positive control)', async () => {
    const event = press(document.body, 'F')
    await nextTick()
    expect(event.defaultPrevented).toBe(true)
    expect(globalSearchVisible()).toBe(true)
  })
})

// Ctrl+Shift+Q / Ctrl+Shift+A must stay quiet while the user is typing.
describe('SqlPage input-state guard', () => {
  function typeTarget(): HTMLInputElement {
    const input = document.createElement('input')
    wrapper!.element.appendChild(input)
    return input
  }

  it('ignores Ctrl+Shift+Q inside a form control', async () => {
    const fromInput = press(typeTarget(), 'Q')
    await nextTick()
    expect(fromInput.defaultPrevented).toBe(false)
    expect(globalQueryVisible()).toBe(false)

    // Page level: the shortcut fires (Global Query itself still needs a
    // connected session with databases before it can show anything).
    const fromPage = press(document.body, 'Q')
    await nextTick()
    expect(fromPage.defaultPrevented).toBe(true)
  })

  it('ignores Ctrl+Shift+A inside a form control', async () => {
    const fromInput = press(typeTarget(), 'A')
    await nextTick()
    expect(fromInput.defaultPrevented).toBe(false)
    expect(aiAssistantVisible()).toBe(false)

    const fromPage = press(document.body, 'A')
    await nextTick()
    expect(fromPage.defaultPrevented).toBe(true)
    expect(aiAssistantVisible()).toBe(true)
  })

  it('ignores Ctrl+Shift+Q inside a <select> form control', async () => {
    const select = document.createElement('select')
    wrapper!.element.appendChild(select)

    const event = press(select, 'Q')
    await nextTick()
    expect(event.defaultPrevented).toBe(false)
    expect(globalQueryVisible()).toBe(false)
  })
})

// The CodeMirror surface is contenteditable, but it is an editor — page-level
// Ctrl+Shift+Q / Ctrl+Shift+A must not be swallowed while the SQL editor is
// focused (the state users are in most of the time).
describe('SqlPage CodeMirror editor focus', () => {
  let cmContent: HTMLElement

  beforeEach(async () => {
    const editorRoot = await openQueryTab(wrapper!)
    // CodeMirror builds its DOM asynchronously after the editor is created.
    await new Promise((resolve) => setTimeout(resolve, 0))
    const content = editorRoot.querySelector('.cm-content')
    expect(content, 'cm-content').toBeTruthy()
    cmContent = content as HTMLElement
  })

  it('fires Ctrl+Shift+Q from the focused editor', async () => {
    expect(cmContent.isContentEditable).toBe(true)

    const event = press(cmContent, 'Q')
    await nextTick()
    expect(event.defaultPrevented).toBe(true)
  })

  it('fires Ctrl+Shift+A from the focused editor', async () => {
    const event = press(cmContent, 'A')
    await nextTick()
    expect(event.defaultPrevented).toBe(true)
    expect(aiAssistantVisible()).toBe(true)
  })

  it('ignores page shortcuts from a CodeMirror outside the SQL editor', async () => {
    // Another pane's CM6 (e.g. a file editor dialog) is contenteditable but
    // not under `.sql-editor`, so the document-level handler must treat it as
    // a typing field and leave the key to the editor instead of popping a
    // page overlay.
    const foreign = document.createElement('div')
    foreign.className = 'cm-content'
    foreign.setAttribute('contenteditable', 'true')
    wrapper!.element.appendChild(foreign)

    const fromForeignQ = press(foreign, 'Q')
    await nextTick()
    expect(fromForeignQ.defaultPrevented).toBe(false)
    expect(globalQueryVisible()).toBe(false)

    const fromForeignF = press(foreign, 'F')
    await nextTick()
    expect(fromForeignF.defaultPrevented).toBe(false)
    expect(globalSearchVisible()).toBe(false)

    foreign.remove()
  })
})

// Split panes render one SqlPage per tab: only the instance owning the active
// pane may react (and preventDefault) to a document-level keystroke.
describe('SqlPage split-pane ownership', () => {
  // PaneCtx stub with only the members SqlPage reads (split-pane ownership).
  function buildPaneCtx(
    leaves: { id: string; tabId: string | null }[],
    activePaneId: string,
    activeTabId: string,
  ): PaneCtx {
    return {
      allLeaves: ref(leaves),
      activePaneId: ref(activePaneId),
      activeTabInfo: computed(() => ({ id: activeTabId, label: 'SQL', protocol: 'sql', status: 'connected' })),
    } as unknown as PaneCtx
  }

  // The file-level default instance has no tabId (always owns keystrokes), so
  // drop it before asserting per-instance ownership.
  function releaseDefaultInstance() {
    wrapper?.unmount()
    wrapper = null
  }

  function mountInPane(tabId: string, ctx: PaneCtx): VueWrapper {
    const w = mount(SqlPage, {
      props: { tabId },
      global: { provide: { [PANE_CTX]: ctx }, stubs: pageStubs },
      attachTo: document.body,
    })
    extraWrappers.push(w)
    return w
  }

  it('opens exactly one overlay, in the instance owning the active pane', async () => {
    releaseDefaultInstance()
    const ctx = buildPaneCtx(
      [{ id: 'pane-1', tabId: 'tab-a' }, { id: 'pane-2', tabId: 'tab-b' }],
      'pane-1',
      'tab-b',
    )
    const a = mountInPane('tab-a', ctx)
    const b = mountInPane('tab-b', ctx)
    await nextTick()

    const event = press(document.body, 'A')
    await nextTick()

    expect(event.defaultPrevented).toBe(true)
    expect(a.find('.ai-stub').attributes('data-visible')).toBe('true')
    expect(b.find('.ai-stub').attributes('data-visible')).toBe('false')
    const openLayers = Array.from(document.querySelectorAll('.ai-stub'))
      .filter(el => el.getAttribute('data-visible') === 'true')
    expect(openLayers).toHaveLength(1)
  })

  it('stays quiet while the active pane is a non-SQL pane', async () => {
    releaseDefaultInstance()
    const ctx = buildPaneCtx(
      [{ id: 'pane-files', tabId: 'tab-files' }, { id: 'pane-sql', tabId: 'tab-sql' }],
      'pane-files',
      'tab-files',
    )
    const sql = mountInPane('tab-sql', ctx)
    await nextTick()

    // The Files pane owns the focus and the target is not a typing field —
    // the SQL page must not hijack the keystroke anyway.
    const fromQ = press(document.body, 'Q')
    await nextTick()
    expect(fromQ.defaultPrevented).toBe(false)
    expect(sql.find('.gq-stub').attributes('data-visible')).toBe('false')

    const fromA = press(document.body, 'A')
    await nextTick()
    expect(fromA.defaultPrevented).toBe(false)
    expect(sql.find('.ai-stub').attributes('data-visible')).toBe('false')

    const fromF = press(document.body, 'F')
    await nextTick()
    expect(fromF.defaultPrevented).toBe(false)
    expect(sql.find('.gs-stub').attributes('data-visible')).toBe('false')
  })
})
