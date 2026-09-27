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
