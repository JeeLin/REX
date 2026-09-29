import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount, type VueWrapper } from '@vue/test-utils'
import { nextTick } from 'vue'
import SqlPage from '../SqlPage.vue'
import SqlEditor from '../SqlEditor.vue'

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
    global: {
      stubs: {
        // SqlEditor stays real: the scoping assertions walk its actual tree.
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
      },
    },
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

beforeEach(async () => {
  wrapper = await mountSqlPage()
})

afterEach(() => {
  wrapper?.unmount()
  wrapper = null
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

  beforeEach(async () => {
    editorRoot = await openQueryTab(wrapper!)
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
})
