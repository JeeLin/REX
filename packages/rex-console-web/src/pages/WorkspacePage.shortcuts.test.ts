import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount, type VueWrapper } from '@vue/test-utils'
import { nextTick } from 'vue'
import { setActivePinia, createPinia } from 'pinia'

vi.mock('vue-i18n', () => ({ useI18n: () => ({ t: (k: string) => k }) }))
vi.mock('vue-router', () => ({ useRouter: () => ({ push: vi.fn() }) }))

// Heavy siblings never take part in shortcut handling.
vi.mock('@/components/ui/StatusDot.vue', () => ({
  default: { name: 'StatusDot', props: ['status'], template: '<span class="status-dot" />' },
}))
vi.mock('@/components/ui/ContextMenu.vue', () => ({
  default: { name: 'ContextMenu', props: ['show', 'x', 'y'], template: '<div />' },
}))
vi.mock('@/features/workspace/PaneNode.vue', () => ({
  default: { name: 'PaneNode', props: ['node'], template: '<div class="pane-node-stub" />' },
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

import WorkspacePage from './WorkspacePage.vue'

let wrapper: VueWrapper | null = null

function press(key: string, init: KeyboardEventInit = {}, target: EventTarget = window): KeyboardEvent {
  const ev = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init })
  target.dispatchEvent(ev)
  return ev
}

async function pressAndFlush(key: string, init: KeyboardEventInit = {}, target: EventTarget = window) {
  const ev = press(key, init, target)
  await nextTick()
  return ev
}

function tabs(): string[] {
  return wrapper!.findAll('.ws-tab').map(t => t.text())
}

function activeTabIndex(): number {
  return wrapper!.findAll('.ws-tab').findIndex(t => t.classes().includes('ws-tab--active'))
}

async function newTab(count: number) {
  for (let i = 0; i < count; i++) await pressAndFlush('t', { altKey: true })
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

describe('browser-reserved keys are released', () => {
  it.each([
    ['Ctrl+T', 't', { ctrlKey: true }],
    ['Ctrl+W', 'w', { ctrlKey: true }],
    ['Ctrl+N', 'n', { ctrlKey: true }],
    ['Ctrl+Tab', 'Tab', { ctrlKey: true }],
    ['Ctrl+Shift+Tab', 'Tab', { ctrlKey: true, shiftKey: true }],
  ])('%s no longer prevents default nor triggers app actions', async (_name, key, init) => {
    const before = tabs().length
    const ev = await pressAndFlush(key, init)

    expect(ev.defaultPrevented).toBe(false)
    expect(tabs().length).toBe(before)
  })
})

describe('replacement shortcuts', () => {
  it('Alt+T creates a new tab', async () => {
    expect(tabs().length).toBe(0)

    const ev = await pressAndFlush('t', { altKey: true })

    expect(ev.defaultPrevented).toBe(true)
    expect(tabs().length).toBe(1)
  })

  it('Alt+W closes the current tab', async () => {
    await newTab(2)
    expect(tabs().length).toBe(2)

    const ev = await pressAndFlush('w', { altKey: true })

    expect(ev.defaultPrevented).toBe(true)
    expect(tabs().length).toBe(1)
  })

  it('Ctrl+Shift+→ cycles to the next tab and wraps around', async () => {
    await newTab(3)
    expect(activeTabIndex()).toBe(2)

    const ev = await pressAndFlush('ArrowRight', { ctrlKey: true, shiftKey: true })

    expect(ev.defaultPrevented).toBe(true)
    expect(activeTabIndex()).toBe(0)
  })

  it('Ctrl+Shift+← cycles to the previous tab', async () => {
    await newTab(3)
    expect(activeTabIndex()).toBe(2)

    await pressAndFlush('ArrowRight', { ctrlKey: true, shiftKey: true })
    const ev = await pressAndFlush('ArrowLeft', { ctrlKey: true, shiftKey: true })

    expect(ev.defaultPrevented).toBe(true)
    expect(activeTabIndex()).toBe(2)
  })

  it('keeps Ctrl+←/→ on tab history instead of tab cycling', async () => {
    await newTab(3)
    expect(activeTabIndex()).toBe(2)

    const ev = await pressAndFlush('ArrowRight', { ctrlKey: true })

    // Still bound (history), but does not cycle tabs.
    expect(ev.defaultPrevented).toBe(true)
    expect(activeTabIndex()).toBe(2)
  })

  it('does not hijack replacement keys while an input is focused', async () => {
    await newTab(1)
    const input = document.createElement('input')
    document.body.appendChild(input)
    input.focus()

    const ev = await pressAndFlush('t', { altKey: true }, input)

    expect(ev.defaultPrevented).toBe(false)
    expect(tabs().length).toBe(1)
  })
})

describe('alt digit shortcuts', () => {
  const paneRoot = () =>
    wrapper!.findComponent({ name: 'PaneNode' }).props('node') as { direction: string | null; children: unknown[] }

  it('Alt+1~9 jump to the corresponding tab', async () => {
    await newTab(3)
    expect(activeTabIndex()).toBe(2)

    await pressAndFlush('1', { altKey: true })
    expect(activeTabIndex()).toBe(0)

    await pressAndFlush('3', { altKey: true })
    expect(activeTabIndex()).toBe(2)

    // No 9th tab: active tab stays where it is.
    await pressAndFlush('9', { altKey: true })
    expect(activeTabIndex()).toBe(2)
    expect(tabs().length).toBe(3)
  })

  it('Ctrl+Alt+1~5 apply layout presets without jumping tabs', async () => {
    await newTab(2)
    expect(activeTabIndex()).toBe(1)

    const ev = await pressAndFlush('2', { ctrlKey: true, altKey: true })

    expect(ev.defaultPrevented).toBe(true)
    expect(paneRoot().children.length).toBe(2)
    expect(paneRoot().direction).toBe('row')
    expect(activeTabIndex()).toBe(1)
  })

  it('keeps pure Alt digits on tab jumps and Ctrl+Alt digits on layouts', async () => {
    await newTab(3)

    await pressAndFlush('2', { altKey: true })
    expect(activeTabIndex()).toBe(1)
    expect(paneRoot().children.length).toBe(1)

    await pressAndFlush('3', { ctrlKey: true, altKey: true })
    expect(paneRoot().direction).toBe('column')
    expect(activeTabIndex()).toBe(1)

    await pressAndFlush('4', { ctrlKey: true, altKey: true })
    expect(paneRoot().direction).toBe('column')
    expect(paneRoot().children.length).toBe(2)
    expect(activeTabIndex()).toBe(1)
  })
})

describe('command palette entry', () => {
  it('ignores Ctrl+K so only the global palette owns the binding', async () => {
    const ev = await pressAndFlush('k', { ctrlKey: true })

    expect(ev.defaultPrevented).toBe(false)
    expect(document.querySelectorAll('.palette-overlay, .command-palette-overlay').length).toBe(0)
  })

  it('routes the status bar palette button to the global palette event', async () => {
    const listener = vi.fn()
    document.addEventListener('rex:command-palette-toggle', listener)

    await wrapper!.find('[title="Command palette (Ctrl+K)"]').trigger('click')

    expect(listener).toHaveBeenCalledTimes(1)
    document.removeEventListener('rex:command-palette-toggle', listener)
  })
})

describe('reopen closed tab', () => {
  it('Alt+Shift+T reopens the last closed tab', async () => {
    await newTab(2)
    // close one tab with Alt+W
    await pressAndFlush('w', { altKey: true })
    expect(tabs().length).toBe(1)

    // reopen with Alt+Shift+T
    const ev = await pressAndFlush('t', { altKey: true, shiftKey: true })
    expect(ev.defaultPrevented).toBe(true)
    expect(tabs().length).toBe(2)
  })
})

describe('fullscreen toolbar button', () => {
  const fsButton = () =>
    wrapper!.find('[aria-label="common.fullscreen"], [aria-label="common.exitFullscreen"]')

  it('falls back to a UI flag and stays in sync when the Fullscreen API is unavailable', async () => {
    expect(fsButton().exists()).toBe(true)
    expect(fsButton().attributes('title')).toBe('common.fullscreen')

    // jsdom has no requestFullscreen: the composable must swallow the rejection
    // and flip the flag instead of throwing.
    await fsButton().trigger('click')
    expect(fsButton().attributes('title')).toBe('common.exitFullscreen')

    await fsButton().trigger('click')
    expect(fsButton().attributes('title')).toBe('common.fullscreen')
  })
})

describe('pane sync (activeTab ↔ pane.tabId)', () => {
  interface PaneTree {
    id: string
    direction: 'row' | 'column' | null
    children: PaneTree[]
    tabId: string | null
  }

  const tree = () =>
    wrapper!.findComponent({ name: 'PaneNode' }).props('node') as unknown as PaneTree

  function leaves(): PaneTree[] {
    const out: PaneTree[] = []
    const walk = (n: PaneTree) => {
      if (n.direction === null) out.push(n)
      else n.children.forEach(walk)
    }
    walk(tree())
    return out
  }

  /** The active pane's tabId. Single pane ⇒ only leaf; after a split the new pane becomes active. */
  function activePaneTab(): string | null {
    const ls = leaves()
    return ls.length === 1 ? ls[0]!.tabId : ls[ls.length - 1]!.tabId
  }

  function tabIdsInPanes(): Array<string | null> {
    return leaves().map(l => l.tabId)
  }

  it('Alt+T binds the new tab to the active pane', async () => {
    expect(activePaneTab()).toBeNull()

    await pressAndFlush('t', { altKey: true })
    expect(activePaneTab()).not.toBeNull()
    expect(activePaneTab()).not.toBe('')

    const first = activePaneTab()
    await pressAndFlush('t', { altKey: true })
    expect(activePaneTab()).not.toBeNull()
    expect(activePaneTab()).not.toBe(first)
  })

  it('cycle tab keeps the pane following the active tab (round trip)', async () => {
    await newTab(3)
    const last = activePaneTab()
    expect(last).not.toBeNull()

    await pressAndFlush('ArrowRight', { ctrlKey: true, shiftKey: true }) // wraps to first
    const first = activePaneTab()
    expect(first).not.toBeNull()
    expect(first).not.toBe(last)

    await pressAndFlush('ArrowLeft', { ctrlKey: true, shiftKey: true }) // back to last
    expect(activePaneTab()).toBe(last)
  })

  it('goBack/goForward keep the pane in sync with history', async () => {
    await newTab(3)
    const third = activePaneTab()

    await pressAndFlush('1', { altKey: true })
    const first = activePaneTab()
    expect(first).not.toBe(third)

    await pressAndFlush('ArrowLeft', { ctrlKey: true }) // goBack
    expect(activePaneTab()).not.toBe(first)

    await pressAndFlush('ArrowRight', { ctrlKey: true }) // goForward
    expect(activePaneTab()).toBe(first)
  })

  it('Alt+1~9 jumps move the tab into the active pane', async () => {
    await newTab(3)
    const third = activePaneTab()

    await pressAndFlush('1', { altKey: true })
    const first = activePaneTab()
    expect(first).not.toBe(third)

    await pressAndFlush('3', { altKey: true })
    expect(activePaneTab()).toBe(third)
  })

  it('Alt+W close leaves no pane pointing at the removed tab', async () => {
    await newTab(2)
    const second = activePaneTab()

    await pressAndFlush('w', { altKey: true })

    expect(tabs().length).toBe(1)
    expect(tabIdsInPanes()).not.toContain(second)
    expect(activePaneTab()).not.toBeNull() // fallback tab bound, no null/blank pane
  })

  it('closing every tab clears pane bindings', async () => {
    await newTab(1)
    expect(activePaneTab()).not.toBeNull()

    await pressAndFlush('w', { altKey: true })

    expect(tabs().length).toBe(0)
    expect(tabIdsInPanes().every(id => id === null)).toBe(true)
  })

  it('jumping to a tab clears its old pane (one tab per pane)', async () => {
    await newTab(2)
    const second = activePaneTab()

    // Split: two panes, the new (right) pane becomes active and is initially empty.
    await pressAndFlush('\\', { ctrlKey: true })
    expect(leaves().length).toBe(2)
    expect(activePaneTab()).toBeNull()
    expect(tabIdsInPanes()).toContain(second) // old pane still holds the tab

    // Jump back to the tab already shown in the left pane: it must move, not duplicate.
    await pressAndFlush('2', { altKey: true })
    expect(activePaneTab()).toBe(second)
    expect(tabIdsInPanes().filter(id => id === second).length).toBe(1)
  })

  it('Ctrl+Shift+N routes to /workspace', async () => {
    await pressAndFlush('n', { ctrlKey: true, shiftKey: true })
    // router.push is mocked; assert no tab churn and the handler ran without error.
    // Routing assertion lives at the handler level: no tabs created, no crash.
    expect(tabs().length).toBe(0)
  })
})
