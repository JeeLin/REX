import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount, flushPromises, type VueWrapper } from '@vue/test-utils'
import { nextTick } from 'vue'
import { setActivePinia, createPinia } from 'pinia'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({ t: (k: string) => k, locale: { value: 'zh' } }),
}))
vi.mock('vue-router', () => ({
  useRouter: () => ({ push: vi.fn(), replace: vi.fn() }),
  useRoute: () => ({ path: '/workspace' }),
  RouterLink: { name: 'RouterLink', props: ['to'], template: '<a><slot /></a>' },
  RouterView: { name: 'RouterView', template: '<div><slot /></div>' },
}))
vi.mock('@/stores/app', () => ({
  useAppStore: () => ({ isHub: true, checkMode: vi.fn() }),
}))
vi.mock('@/stores/auth', () => ({
  useAuthStore: () => ({ logout: vi.fn(), isAuthenticated: true }),
}))
vi.mock('@/features/resource-panel/ResourcePanel.vue', () => ({
  default: { name: 'ResourcePanel', template: '<div class="resource-panel-stub" />' },
}))
vi.mock('@/features/workspace/ShortcutPanel.vue', () => ({
  default: { name: 'ShortcutPanel', props: ['show'], template: '<div v-if="show" class="shortcut-panel-stub" />' },
}))
vi.mock('@/composables/useSessionTimeout', async () => {
  const { ref } = await import('vue')
  return {
    useSessionTimeout: () => ({ showWarning: ref(false), remainingSeconds: ref(0), extendSession: vi.fn() }),
  }
})

import AppLayout from './AppLayout.vue'

let wrapper: VueWrapper | null = null

let fullscreenEnabled = true
let fullscreenElement: Element | null = null
const requestFullscreen = vi.fn()
const exitFullscreen = vi.fn()

function press(key: string, init: KeyboardEventInit = {}) {
  const ev = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init })
  document.dispatchEvent(ev)
  return ev
}

async function pressF11() {
  press('F11')
  await flushPromises()
}

function fullscreenClass(): boolean {
  return wrapper!.find('.app-layout').classes().includes('app-layout--fullscreen')
}

function paletteCount(): number {
  return document.querySelectorAll('.command-palette-overlay').length
}

async function togglePalette() {
  document.dispatchEvent(new CustomEvent('rex:command-palette-toggle'))
  await nextTick()
}

beforeEach(() => {
  fullscreenEnabled = true
  fullscreenElement = null
  requestFullscreen.mockReset().mockImplementation(async () => {
    fullscreenElement = document.documentElement
  })
  exitFullscreen.mockReset().mockImplementation(async () => {
    fullscreenElement = null
  })
  Object.defineProperty(document, 'fullscreenEnabled', { configurable: true, get: () => fullscreenEnabled })
  Object.defineProperty(document, 'fullscreenElement', { configurable: true, get: () => fullscreenElement })
  document.documentElement.requestFullscreen = requestFullscreen
  document.exitFullscreen = exitFullscreen

  setActivePinia(createPinia())
  wrapper = mount(AppLayout)
})

afterEach(() => {
  wrapper?.unmount()
  wrapper = null
})

describe('single command palette entry', () => {
  it('opens exactly one palette on Ctrl+K and closes it on the second press', async () => {
    press('k', { ctrlKey: true })
    await nextTick()
    expect(paletteCount()).toBe(1)

    press('k', { ctrlKey: true })
    await nextTick()
    expect(paletteCount()).toBe(0)
  })

  it('routes the workspace toolbar entry to the same global palette', async () => {
    await togglePalette()
    expect(paletteCount()).toBe(1)

    await togglePalette()
    expect(paletteCount()).toBe(0)
  })

  it('keeps AppLayout as the only Ctrl+K keydown binding in the workspace page pair', () => {
    const sources = import.meta.glob(['../pages/WorkspacePage.vue', './AppLayout.vue'], {
      query: '?raw',
      import: 'default',
      eager: true,
    })
    const workspaceSource = sources['../pages/WorkspacePage.vue'] as string
    const layoutSource = sources['./AppLayout.vue'] as string

    expect(workspaceSource).not.toContain("e.key === 'k'")
    expect(workspaceSource).not.toContain('showCommandPalette')
    expect(layoutSource.match(/e\.key === 'k'/g)).toHaveLength(1)
  })
})

describe('F11 fullscreen', () => {
  it('requests real fullscreen through the Fullscreen API', async () => {
    await pressF11()

    expect(requestFullscreen).toHaveBeenCalledTimes(1)
    expect(exitFullscreen).not.toHaveBeenCalled()
    expect(fullscreenElement).toBe(document.documentElement)
    expect(fullscreenClass()).toBe(true)
    expect(wrapper!.find('.exit-fullscreen-btn').exists()).toBe(true)
  })

  it('exits fullscreen through the Fullscreen API', async () => {
    fullscreenElement = document.documentElement
    document.dispatchEvent(new Event('fullscreenchange'))
    await nextTick()
    expect(fullscreenClass()).toBe(true)

    await pressF11()

    expect(exitFullscreen).toHaveBeenCalledTimes(1)
    expect(requestFullscreen).not.toHaveBeenCalled()
    expect(fullscreenElement).toBe(null)
    expect(fullscreenClass()).toBe(false)
  })

  it('follows document fullscreen state on fullscreenchange', async () => {
    fullscreenElement = document.documentElement
    document.dispatchEvent(new Event('fullscreenchange'))
    await nextTick()
    expect(fullscreenClass()).toBe(true)

    fullscreenElement = null
    document.dispatchEvent(new Event('fullscreenchange'))
    await nextTick()
    expect(fullscreenClass()).toBe(false)
  })

  it('falls back to the Vue flag when the API is disabled', async () => {
    fullscreenEnabled = false

    await pressF11()

    expect(requestFullscreen).not.toHaveBeenCalled()
    expect(exitFullscreen).not.toHaveBeenCalled()
    expect(fullscreenClass()).toBe(true)

    await pressF11()
    expect(fullscreenClass()).toBe(false)
  })

  it('falls back to the Vue flag when the API rejects', async () => {
    requestFullscreen.mockImplementation(async () => {
      throw new Error('not allowed')
    })

    await pressF11()

    expect(requestFullscreen).toHaveBeenCalledTimes(1)
    expect(fullscreenClass()).toBe(true)
  })
})
