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
