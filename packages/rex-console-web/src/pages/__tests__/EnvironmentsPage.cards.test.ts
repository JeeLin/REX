import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import type { Environment } from '@/api/environments'
import zh from '@/i18n/locales/zh.json'
import en from '@/i18n/locales/en.json'

// The real composer (not the stub below) is what resolves `{count} | {count}`,
// so a typo in the message only surfaces in the last describe block.
const { createI18n } = await vi.importActual<typeof import('vue-i18n')>('vue-i18n')

// The `t(key, plural, named)` overload renders the plural form of a `{count}`
// key. The mock keeps the shape of vue-i18n's real output so the badge text
// carries the number the backend sent.
vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string, ...args: unknown[]) => {
      const named = args.find(a => a && typeof a === 'object') as { named?: unknown } | undefined
      return named?.named ? `${key} ${JSON.stringify(named.named)}` : key
    },
  }),
}))

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

function makeEnv(over: Partial<Environment> = {}): Environment {
  return {
    id: 'env-1',
    name: 'Prod',
    description: '',
    connection_mode: 'agent',
    resource_count: 0,
    agent_status: null,
    agents_online: 0,
    registration_token: 'tok',
    created_at: '',
    updated_at: '',
    ...over,
  }
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

beforeEach(() => {
  vi.clearAllMocks()
  envStore.environments = []
  envStore.envResources = new Map()
  envStore.fetchEnvironments.mockResolvedValue(undefined)
  envStore.fetchResources.mockResolvedValue([])
})

describe('EnvironmentsPage environment card', () => {
  it('keeps the action bar immediately after the flex-fill spacer', async () => {
    envStore.environments = [makeEnv()]
    const wrapper = await mountPage()

    const children = Array.from(card(wrapper).element.children) as HTMLElement[]
    const spacer = children.findIndex(c => c.classList.contains('env-card-fill'))
    const actions = children.findIndex(c => c.classList.contains('env-card-actions'))

    // The filler is what pins the footer down; the footer's own `margin-top: auto`
    // must not be the only thing holding it there. It stays immediately before the
    // actions so the free space lands between them rather than after the actions.
    expect(spacer).toBeGreaterThan(-1)
    expect(actions).toBeGreaterThan(-1)
    expect(actions).toBe(spacer + 1)
  })

  it('renders the action bar for an agent environment with no online agent', async () => {
    // agent_status is non-null here on purpose: the footer must not depend on
    // whether an agent is connected.
    envStore.environments = [makeEnv({ agent_status: 'offline', agents_online: 0 })]
    const wrapper = await mountPage()

    const actions = card(wrapper).find('.env-card-actions')
    expect(actions.exists()).toBe(true)
    expect(actions.findAll('.env-card-action')).toHaveLength(4)
    // The filler sits above the footer even when the agent section is absent data.
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
    const badge = wrapper.find('.env-card-agents .badge-green')
    expect(badge.exists()).toBe(true)
    expect(badge.text()).toContain('3')
  })

  it('uses the backend count rather than inferring one from agent_status', async () => {
    // A single offline agent plus two online ones: `agent_status` is 'online'
    // (any-online rule), so deriving the count from it would give 1.
    envStore.environments = [makeEnv({ agent_status: 'online', agents_online: 2 })]
    const wrapper = await mountPage()
    const badge = wrapper.find('.env-card-agents .badge-green')
    expect(badge.text()).toContain('2')
    expect(badge.text()).not.toContain('"count":1')
  })
})

describe('environments.agentsOnlineCount message', () => {
  function render(locale: 'zh' | 'en', n: number): string {
    const i18n = createI18n({ legacy: false, locale, fallbackLocale: 'en', messages: { zh, en } })
    const g = i18n.global as unknown as {
      t: (k: string, p: number, o: Record<string, unknown>) => string
    }
    return g.t('environments.agentsOnlineCount', n, { named: { count: n } })
  }

  it('interpolates the count through the pipe-form message', () => {
    expect(render('en', 3)).toBe('3 online')
    expect(render('zh', 12)).toBe('12 个在线 Agent')
  })
})