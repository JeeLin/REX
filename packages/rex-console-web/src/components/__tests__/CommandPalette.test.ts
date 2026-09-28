import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount, type VueWrapper } from '@vue/test-utils'
import { nextTick } from 'vue'

// Shared state for the mocked i18n/router modules: factories run during import,
// so they must read values captured by vi.hoisted.
const h = vi.hoisted(() => ({
  locale: { value: 'en' },
  push: [] as string[],
}))

vi.mock('vue-i18n', () => ({
  useI18n: () => ({ t: (k: string) => k, locale: h.locale }),
}))
vi.mock('vue-router', () => ({
  useRouter: () => ({ push: (path: string) => h.push.push(path) }),
}))
vi.mock('@/stores/environments', () => ({
  useEnvironmentsStore: () => ({
    environments: [
      { id: 'env-1', name: 'Production', description: 'Prod environment', connection_mode: 'agent' },
    ],
    envResources: new Map(),
    fetchEnvironments: vi.fn(),
  }),
}))

import CommandPalette from '../CommandPalette.vue'

let wrapper: VueWrapper | null = null

async function mountPalette() {
  wrapper?.unmount()
  wrapper = mount(CommandPalette, {
    props: { visible: true },
    global: {
      stubs: { teleport: true },
    },
  })
  await nextTick()
  return wrapper
}

// Search by the i18n-key title (the mock passes keys through) and run the only match.
async function runCommand(titleKey: string) {
  const w = await mountPalette()
  await w.find('.command-palette-input').setValue(titleKey)
  const items = w.findAll('.command-palette-item')
  expect(items).toHaveLength(1)
  await items[0]!.trigger('click')
}

function titles(w: VueWrapper): string[] {
  return w.findAll('.command-palette-item-title').map(el => el.text())
}

beforeEach(() => {
  h.push.length = 0
  h.locale.value = 'en'
  localStorage.clear()
  delete document.documentElement.dataset.theme
})

afterEach(() => {
  wrapper?.unmount()
  wrapper = null
  document.body.innerHTML = ''
})

describe('merged workspace palette commands', () => {
  it('lists the merged navigation, command and setting entries', async () => {
    const w = await mountPalette()

    await w.find('.command-palette-input').setValue('nav.')
    expect(titles(w)).toEqual([
      'nav.workspace',
      'nav.dashboard',
      'nav.environments',
      'nav.agents',
      'nav.auditLog',
      'nav.settings',
    ])

    await w.find('.command-palette-input').setValue('commandPalette.newConnection')
    expect(titles(w)).toEqual(['commandPalette.newConnection'])

    await w.find('.command-palette-input').setValue('switch')
    expect(titles(w)).toEqual([
      'commandPalette.themeDark',
      'commandPalette.themeLight',
      'commandPalette.languageEn',
      'commandPalette.languageZh',
    ])
  })

  it('routes nav-agents to /agents', async () => {
    await runCommand('nav.agents')

    expect(h.push).toEqual(['/agents'])
  })

  it('routes new-connection to the workspace instead of the dead /resource-new route', async () => {
    await runCommand('commandPalette.newConnection')

    expect(h.push).toEqual(['/workspace'])
    expect(h.push).not.toContain('/resource-new')
  })

  it('applies the dark and light theme modes', async () => {
    await runCommand('commandPalette.themeDark')
    expect(localStorage.getItem('rex-theme')).toBe('dark')
    expect(document.documentElement.dataset.theme).toBeUndefined()

    await runCommand('commandPalette.themeLight')
    expect(localStorage.getItem('rex-theme')).toBe('light')
    expect(document.documentElement.dataset.theme).toBe('light')
  })

  it('switches the locale and persists it', async () => {
    await runCommand('commandPalette.languageZh')
    expect(h.locale.value).toBe('zh')
    expect(localStorage.getItem('rex-lang')).toBe('zh')

    await runCommand('commandPalette.languageEn')
    expect(h.locale.value).toBe('en')
    expect(localStorage.getItem('rex-lang')).toBe('en')
  })
})
