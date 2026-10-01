import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount, type VueWrapper } from '@vue/test-utils'
import { nextTick } from 'vue'
import { setActivePinia, createPinia } from 'pinia'

vi.mock('vue-i18n', () => ({ useI18n: () => ({ t: (k: string) => k }) }))
vi.mock('vue-router', () => ({ useRouter: () => ({ push: vi.fn() }) }))

// Heavy siblings never take part in drag & drop handling.
vi.mock('@/components/ui/StatusDot.vue', () => ({
  default: { name: 'StatusDot', props: ['status'], template: '<span class="status-dot" />' },
}))
vi.mock('@/components/ui/ContextMenu.vue', () => ({
  default: { name: 'ContextMenu', props: ['show', 'x', 'y'], template: '<div />' },
}))
vi.mock('@/features/workspace/WelcomePage.vue', () => ({
  default: { name: 'WelcomePage', template: '<div class="welcome-stub" />' },
}))
vi.mock('@/features/workspace/ResourceProperties.vue', () => ({
  default: { name: 'ResourceProperties', props: ['show', 'resource'], template: '<div />' },
}))
vi.mock('@/features/workspace/ToolbarSettings.vue', () => ({
  default: { name: 'ToolbarSettings', template: '<div />' },
}))
// Leaves render the real PaneLeaf so the native drop wiring under test is the
// production one; containers recurse without pulling in splitpanes.
vi.mock('@/features/workspace/PaneNode.vue', async () => {
  const { h } = await import('vue')
  const { default: PaneLeaf } = await import('@/features/workspace/PaneLeaf.vue')
  const PaneNodeStub = {
    name: 'PaneNode',
    props: ['node'],
    setup(props: { node: PaneTreeNode }) {
      return () =>
        props.node.direction === null
          ? h(PaneLeaf, { leafId: props.node.id })
          : h(
              'div',
              { class: 'pane-node-group' },
              props.node.children.map((child) => h(PaneNodeStub, { node: child })),
            )
    },
  }
  return { default: PaneNodeStub }
})
// PaneLeaf's own children are covered by their own tests.
vi.mock('@/features/terminal/WorkspaceTerminal.vue', () => ({
  default: { name: 'WorkspaceTerminal', props: ['tabId', 'resourceId', 'name'], template: '<div class="ws-terminal-stub" />' },
}))
vi.mock('@/features/files/FilesDrawer.vue', () => ({
  default: { name: 'FilesDrawer', props: ['resourceId'], template: '<div class="ws-sftp-drawer-stub" />' },
}))

import WorkspacePage from './WorkspacePage.vue'

interface PaneTreeNode {
  id: string
  direction: 'row' | 'column' | null
  children: PaneTreeNode[]
  tabId: string | null
}

let wrapper: VueWrapper | null = null

function press(key: string, init: KeyboardEventInit = {}, target: EventTarget = document.body): KeyboardEvent {
  const ev = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init })
  target.dispatchEvent(ev)
  return ev
}

async function pressAndFlush(key: string, init: KeyboardEventInit = {}, target: EventTarget = document.body) {
  const ev = press(key, init, target)
  await nextTick()
  return ev
}

function paneTree(): PaneTreeNode {
  return wrapper!.findComponent({ name: 'PaneNode' }).props('node') as PaneTreeNode
}

function leaves(node: PaneTreeNode = paneTree()): PaneTreeNode[] {
  return node.direction === null ? [node] : node.children.flatMap(leaves)
}

function panes() {
  return wrapper!.findAll('.ws-pane')
}

// Native drag events, mirroring what the browser delivers to PaneLeaf's
// @dragover / @drop handlers (the drag payload rides on dataTransfer).
async function dragOver(paneIndex: number) {
  panes()[paneIndex]!.element.dispatchEvent(new Event('dragover', { bubbles: true, cancelable: true }))
  await nextTick()
}

async function drop(paneIndex: number, tabId: string) {
  const event = new Event('drop', { bubbles: true, cancelable: true })
  Object.defineProperty(event, 'dataTransfer', {
    value: { getData: (type: string) => (type === 'text/tab-id' ? tabId : '') },
  })
  panes()[paneIndex]!.element.dispatchEvent(event)
  await nextTick()
}

beforeEach(() => {
  localStorage.clear()
  setActivePinia(createPinia())
  wrapper = mount(WorkspacePage)
})

afterEach(() => {
  wrapper?.unmount()
  wrapper = null
  document.body.innerHTML = ''
})

describe('pane drag & drop', () => {
  it('drops a tab onto another pane and focuses the target pane', async () => {
    // One pane holding a freshly opened tab.
    await pressAndFlush('t', { altKey: true })
    expect(leaves()).toHaveLength(1)
    const tabId = leaves()[0]!.tabId!
    expect(tabId).toBeTruthy()

    // Split: splitPane marks the new pane active while lastFocusedPaneId keeps
    // pointing at the pane that holds the tab — the state drop must reconcile.
    await pressAndFlush('\\', { ctrlKey: true, code: 'Backslash' })
    expect(leaves()).toHaveLength(2)
    const [source, target] = leaves() as [PaneTreeNode, PaneTreeNode]
    expect(source.tabId).toBe(tabId)
    expect(target.tabId).toBe(null)
    expect(panes()[1]!.classes()).toContain('ws-pane--active')

    // Focus the source pane so the drop target is NOT the active pane — the
    // pre-drop highlight must diverge from the target for the post-drop
    // assertion to prove drop actually moves activePaneId.
    await panes()[0]!.trigger('click')
    expect(panes()[0]!.classes()).toContain('ws-pane--active')
    expect(panes()[1]!.classes()).not.toContain('ws-pane--active')

    await dragOver(1)
    expect(panes()[1]!.classes()).toContain('ws-pane--drag-over')

    await drop(1, tabId)

    // The tab moved out of the source pane and onto the drop target.
    expect(leaves()[0]!.tabId).toBe(null)
    expect(leaves()[1]!.tabId).toBe(tabId)
    expect(panes()[1]!.classes()).not.toContain('ws-pane--drag-over')
    // The active highlight migrated onto the drop target and left the source.
    expect(panes().findIndex((p) => p.classes().includes('ws-pane--active'))).toBe(1)
    expect(panes()[1]!.classes()).toContain('ws-pane--active')
    expect(panes()[0]!.classes()).not.toContain('ws-pane--active')

    // lastFocusedPaneId points at the drop target too: the next shortcut split
    // must land after it instead of after the stale pre-drop focus.
    await pressAndFlush('\\', { ctrlKey: true, code: 'Backslash' })
    const ids = leaves().map((l) => l.id)
    expect(ids).toHaveLength(3)
    const newPaneId = ids.find((id) => id !== source.id && id !== target.id)!
    expect(ids.indexOf(newPaneId)).toBe(2)
  })

  it('clears the drag-over highlight when the drop carries no tab payload', async () => {
    await pressAndFlush('t', { altKey: true })
    await pressAndFlush('\\', { ctrlKey: true, code: 'Backslash' })
    const bindingsBefore = leaves().map((l) => l.tabId)

    await dragOver(1)
    expect(panes()[1]!.classes()).toContain('ws-pane--drag-over')

    await drop(1, '')

    expect(leaves().map((l) => l.tabId)).toEqual(bindingsBefore)
    expect(panes()[1]!.classes()).not.toContain('ws-pane--drag-over')
    expect(panes()[1]!.classes()).toContain('ws-pane--active')
  })
})
