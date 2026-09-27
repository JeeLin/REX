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

interface PaneNode {
  direction: 'row' | 'column' | null
  children: PaneNode[]
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

function paneRoot(): PaneNode {
  return wrapper!.findComponent({ name: 'PaneNode' }).props('node') as PaneNode
}

function leaves(node: PaneNode): PaneNode[] {
  return node.direction === null ? [node] : node.children.flatMap(leaves)
}

beforeEach(() => {
  localStorage.clear()
  setActivePinia(createPinia())
  wrapper = mount(WorkspacePage)
})

afterEach(() => {
  wrapper?.unmount()
  wrapper = null
})

describe('split shortcuts', () => {
  it('Ctrl+Shift+\\ splits the active pane vertically', async () => {
    expect(leaves(paneRoot()).length).toBe(1)

    // Shift turns the character into '|' while the physical key stays Backslash.
    const ev = await pressAndFlush('|', { ctrlKey: true, shiftKey: true, code: 'Backslash' })

    expect(ev.defaultPrevented).toBe(true)
    expect(leaves(paneRoot()).length).toBe(2)
    expect(paneRoot().children).toHaveLength(1)
    expect(paneRoot().children[0]!.direction).toBe('column')
  })

  it('splits exactly once regardless of the reported key character', async () => {
    await pressAndFlush('\\', { ctrlKey: true, shiftKey: true, code: 'Backslash' })

    expect(leaves(paneRoot()).length).toBe(2)
  })

  it('Ctrl+\\ splits the active pane horizontally', async () => {
    const ev = await pressAndFlush('\\', { ctrlKey: true, code: 'Backslash' })

    expect(ev.defaultPrevented).toBe(true)
    expect(leaves(paneRoot()).length).toBe(2)
    expect(paneRoot().direction).toBe('row')
    expect(paneRoot().children).toHaveLength(2)
  })

  it('does not split while an input is focused', async () => {
    const input = document.createElement('input')
    document.body.appendChild(input)
    input.focus()

    const ev = await pressAndFlush('\\', { ctrlKey: true, shiftKey: true, code: 'Backslash' }, input)

    expect(ev.defaultPrevented).toBe(false)
    expect(leaves(paneRoot()).length).toBe(1)
  })
})
