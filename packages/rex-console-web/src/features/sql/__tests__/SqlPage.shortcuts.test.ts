import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount, type VueWrapper } from '@vue/test-utils'
import { nextTick } from 'vue'
import SqlPage from '../SqlPage.vue'

vi.mock('vue-i18n', () => ({
  // SqlPage reads `locale.value` for the format tooltip copy.
  useI18n: () => ({
    t: (k: string) => k,
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

// Ctrl+Shift+F is context-scoped: SQL editor focused = format (handled inside
// the editor), SQL page level = global search.
describe('SqlPage Ctrl+Shift+F scoping', () => {
  let wrapper: VueWrapper | null = null

  function pressCtrlShiftF(target: EventTarget): KeyboardEvent {
    const event = new KeyboardEvent('keydown', {
      key: 'F',
      ctrlKey: true,
      shiftKey: true,
      cancelable: true,
      bubbles: true,
    })
    target.dispatchEvent(event)
    return event
  }

  function globalSearchVisible(): boolean {
    return wrapper!.find('.gs-stub').attributes('data-visible') === 'true'
  }

  beforeEach(() => {
    wrapper = mount(SqlPage, {
      global: {
        stubs: {
          SqlEditor: true,
          SqlResultGrid: true,
          TableDesigner: true,
          ExportWizard: true,
          GlobalQueryModal: true,
          AiAssistantDrawer: true,
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
  })

  afterEach(() => {
    wrapper?.unmount()
    wrapper = null
    document.querySelectorAll('.sql-editor').forEach((el) => el.remove())
  })

  it('yields to the SQL editor instead of opening global search', async () => {
    const host = document.createElement('div')
    host.className = 'sql-editor'
    const inner = document.createElement('span')
    host.appendChild(inner)
    document.body.appendChild(host)

    // Keydown from a descendant of the editor must not be hijacked.
    const fromInner = pressCtrlShiftF(inner)
    await nextTick()
    expect(fromInner.defaultPrevented).toBe(false)
    expect(globalSearchVisible()).toBe(false)

    // Keydown directly on the editor root behaves the same.
    const fromRoot = pressCtrlShiftF(host)
    await nextTick()
    expect(fromRoot.defaultPrevented).toBe(false)
    expect(globalSearchVisible()).toBe(false)
  })

  it('opens global search at SQL page level (positive control)', async () => {
    const event = pressCtrlShiftF(document.body)
    await nextTick()
    expect(event.defaultPrevented).toBe(true)
    expect(globalSearchVisible()).toBe(true)
  })
})
