import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import type { Environment } from '@/api/environments'
import { makeEnv } from '@/stores/__tests__/makeEnv'
import zh from '@/i18n/locales/zh.json'
import en from '@/i18n/locales/en.json'

// The page renders through the *real* composer over the real message files, so a
// badge assertion reads what the user reads ("2 online") instead of a test
// double's private serialization. Only `useI18n` is stubbed; the last describe
// below drives the real composer directly to pin the message itself.
vi.mock('vue-i18n', async () => {
  const actual = await vi.importActual<typeof import('vue-i18n')>('vue-i18n')
  const messages = (await import('@/i18n/locales/en.json')).default
  const i18n = actual.createI18n({
    legacy: false,
    locale: 'en',
    fallbackLocale: 'en',
    messages: { en: messages },
  })
  return { ...actual, useI18n: () => ({ t: i18n.global.t }) }
})

const { createI18n } = await vi.importActual<typeof import('vue-i18n')>('vue-i18n')

const { envStore } = vi.hoisted(() => ({
  envStore: {
    environments: [] as Environment[],
    envResources: new Map<string, unknown[]>(),
    loading: false,
    error: null,
    fetchEnvironments: vi.fn(),
    fetchResources: vi.fn(),
  },
}))

vi.mock('@/stores/environments', () => ({ useEnvironmentsStore: () => envStore }))
vi.mock('@/api/environments', () => ({ environmentsApi: { export: vi.fn(), import: vi.fn() } }))
vi.mock('vue-router', () => ({
  useRouter: () => ({ push: vi.fn(), replace: vi.fn() }),
}))

import EnvironmentsPage from '../EnvironmentsPage.vue'
import pageSource from '../EnvironmentsPage.vue?raw'

/** The component's own stylesheet, read from source: happy-dom runs no layout. */
const pageCss = pageSource.slice(pageSource.indexOf('<style'))

/** Body of a single rule in the component's stylesheet, '' when it has none. */
function cssRule(selector: string): string {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  return pageCss.match(new RegExp(`${escaped}\\s*\\{([^}]*)\\}`))?.[1] ?? ''
}

async function mountPage() {
  const wrapper = mount(EnvironmentsPage, { attachTo: document.body })
  for (let i = 0; i < 4; i++) await flushPromises()
  return wrapper
}

/** The card for a single environment, ignoring the trailing "new environment" card. */
function card(wrapper: ReturnType<typeof mount>) {
  return wrapper.findAll('.env-card').find(c => !c.classes().includes('env-card--new'))!
}

/** The environment card's children, in DOM order. */
function cardChildren(wrapper: ReturnType<typeof mount>): HTMLElement[] {
  return Array.from(card(wrapper).element.children) as HTMLElement[]
}

/** The toolbar's three count badges: environments, online agents, resources. */
function toolbarCounts(wrapper: ReturnType<typeof mount>): string[] {
  return wrapper.findAll('.toolbar .badge-item').map(b => b.text())
}

beforeEach(() => {
  vi.clearAllMocks()
  envStore.environments = []
  envStore.envResources = new Map()
  envStore.fetchEnvironments.mockResolvedValue(undefined)
  envStore.fetchResources.mockResolvedValue([])
})

describe('EnvironmentsPage environment card', () => {
  it('pins the footer with one mechanism only: the flex filler', () => {
    const fillerCss = cssRule('.env-card-fill')
    expect(fillerCss).not.toBe('')
    // The filler grows into the leftover card height, which is what pushes the
    // action bar down to the card's bottom edge.
    expect(fillerCss).toMatch(/flex:\s*1\s+1\s+auto/)

    // A second mechanism here (an auto margin on the bar) would leave the layout
    // depending on which of the two happened to win the free space.
    const actionsCss = cssRule('.env-card-actions')
    expect(actionsCss).not.toBe('')
    expect(actionsCss).not.toMatch(/margin-top\s*:\s*auto/)
  })

  it('keeps the filler above the footer and the footer last', async () => {
    envStore.environments = [makeEnv()]
    const wrapper = await mountPage()

    const children = cardChildren(wrapper)
    const filler = children.findIndex(c => c.classList.contains('env-card-fill'))
    const actions = children.findIndex(c => c.classList.contains('env-card-actions'))

    expect(filler).toBeGreaterThan(-1)
    // The filler must sit below the content it separates from the footer, and the
    // footer must be the card's last child — anything after it would be pushed
    // below the pinned bar.
    expect(filler).toBeLessThan(actions)
    expect(actions).toBe(children.length - 1)
  })

  it('renders the action bar for an agent environment with no online agent', async () => {
    // agent_status is non-null here on purpose: the footer must not depend on
    // whether an agent is connected.
    envStore.environments = [makeEnv({ agent_status: 'offline', agents_online: 0 })]
    const wrapper = await mountPage()

    const actions = card(wrapper).find('.env-card-actions')
    expect(actions.exists()).toBe(true)
    expect(actions.findAll('.env-card-action')).toHaveLength(4)
    // The filler is still rendered even when no agent is online.
    expect(card(wrapper).find('.env-card-fill').exists()).toBe(true)
  })

  it('renders no agent section for a direct environment', async () => {
    envStore.environments = [makeEnv({ connection_mode: 'direct' })]
    const wrapper = await mountPage()

    expect(wrapper.find('.env-card-agents').exists()).toBe(false)
    expect(wrapper.find('.env-card-fill').exists()).toBe(true)
    expect(card(wrapper).find('.env-card-actions').exists()).toBe(true)
  })

  it('hides the online badge at zero and shows the real count above zero', async () => {
    envStore.environments = [makeEnv({ agent_status: 'offline', agents_online: 0 })]
    let wrapper = await mountPage()
    expect(wrapper.find('.env-card-agents .badge-green').exists()).toBe(false)
    wrapper.unmount()

    // 3 online agents must read 3, never a hardcoded 1.
    envStore.environments = [makeEnv({ agent_status: 'online', agents_online: 3 })]
    wrapper = await mountPage()
    expect(wrapper.find('.env-card-agents .badge-green').text()).toBe('3 online')
  })

  it('uses the backend count rather than inferring one from agent_status', async () => {
    // A single offline agent plus two online ones: `agent_status` is 'online'
    // (any-online rule), so deriving the count from it would read "1 online".
    envStore.environments = [makeEnv({ agent_status: 'online', agents_online: 2 })]
    const wrapper = await mountPage()
    expect(wrapper.find('.env-card-agents .badge-green').text()).toBe('2 online')
  })
})

describe('EnvironmentsPage online agent total', () => {
  it('counts agents, not the environments that happen to have one online', async () => {
    envStore.environments = [
      makeEnv({ id: 'env-1', agent_status: 'online', agents_online: 2 }),
      makeEnv({ id: 'env-2', agent_status: 'online', agents_online: 1 }),
      makeEnv({ id: 'env-3', agent_status: 'offline', agents_online: 0 }),
    ]
    const wrapper = await mountPage()

    // 3 agents in 2 environments: counting environments would report 2, and the
    // per-card badges on this same screen sum to 3.
    expect(toolbarCounts(wrapper)[1]).toBe('3 Agents Online')
    expect(wrapper.findAll('.env-card-agents .badge-green').map(b => b.text()))
      .toEqual(['2 online', '1 online'])
  })

  it('reports zero when no agent is online', async () => {
    envStore.environments = [makeEnv({ agent_status: 'offline', agents_online: 0 })]
    const wrapper = await mountPage()
    expect(toolbarCounts(wrapper)[1]).toBe('0 Agents Online')
  })
})

describe('environments.agentsOnlineBadge message', () => {
  function render(locale: 'zh' | 'en', n: number): string {
    const i18n = createI18n({ legacy: false, locale, fallbackLocale: 'en', messages: { zh, en } })
    const g = i18n.global as unknown as {
      t: (k: string, named: Record<string, unknown>) => string
    }
    return g.t('environments.agentsOnlineBadge', { count: n })
  }

  it('interpolates the count', () => {
    expect(render('en', 3)).toBe('3 online')
    expect(render('zh', 12)).toBe('12 个在线 Agent')
  })

  it('carries a single message, not a plural form with two identical branches', () => {
    // vue-i18n silently picks one branch when no plural count is passed, so an
    // identical-branch pipe form renders fine and would look like a deliberate
    // singular/plural split. Both message files must stay pipe-free.
    for (const messages of [en, zh]) {
      const message = messages.environments.agentsOnlineBadge
      expect(message).not.toContain('|')
      expect(message.match(/\{count\}/g)).toHaveLength(1)
    }
    for (const locale of ['en', 'zh'] as const) {
      const rendered = render(locale, 3)
      expect(rendered).not.toContain('|')
      expect(rendered.match(/\d+/g)).toHaveLength(1)
    }
  })
})