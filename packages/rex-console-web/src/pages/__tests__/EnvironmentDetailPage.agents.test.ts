import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { defineComponent, type PropType } from 'vue'
import { makeEnv } from '@/stores/__tests__/makeEnv'

// Real composer over the real message files: the badge assertion reads the text
// the user reads ("2 online"), not a test double's serialization.
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

const { envStore, wsStore, mockGet, mockPut } = vi.hoisted(() => ({
  envStore: {
    envResources: new Map<string, unknown[]>(),
    fetchResources: vi.fn(),
    updateEnvironment: vi.fn(),
    deleteEnvironment: vi.fn(),
    deleteResource: vi.fn(),
    updateResource: vi.fn(),
    testConnection: vi.fn(),
  },
  wsStore: { openResource: vi.fn() },
  mockGet: vi.fn(),
  mockPut: vi.fn(),
}))

vi.mock('@/stores/environments', () => ({ useEnvironmentsStore: () => envStore }))
vi.mock('@/stores/workspace', () => ({ useWorkspaceStore: () => wsStore }))
vi.mock('@/api/environments', () => ({ environmentsApi: { get: mockGet } }))
vi.mock('@/api/client', () => ({ api: { get: mockPut, put: mockPut, post: mockPut, delete: mockPut } }))
vi.mock('vue-router', () => ({
  useRoute: () => ({ params: { id: 'env-1' } }),
  useRouter: () => ({ push: vi.fn(), replace: vi.fn() }),
  onBeforeRouteUpdate: vi.fn(),
}))

import EnvironmentDetailPage from '../EnvironmentDetailPage.vue'

const RouterLinkStub = defineComponent({
  props: { to: { type: [String, Object] as PropType<string | Record<string, unknown>>, default: '' } },
  template: '<a><slot /></a>',
})

async function mountPage() {
  const wrapper = mount(EnvironmentDetailPage, {
    global: { stubs: { 'router-link': RouterLinkStub } },
  })
  for (let i = 0; i < 4; i++) await flushPromises()
  return wrapper
}

/** The Agents section, located by its (hard-coded) heading rather than by index. */
function agentsSection(wrapper: ReturnType<typeof mount>) {
  const section = wrapper.findAll('.section')
    .find(s => s.find('.section-title').text() === 'Agents')
  expect(section, 'Agents section not rendered').toBeTruthy()
  return section!
}

/** The "N online" badge in the Agents section header. */
function onlineBadge(wrapper: ReturnType<typeof mount>) {
  return agentsSection(wrapper).find('.section-head .badge')
}

beforeEach(() => {
  vi.clearAllMocks()
  envStore.envResources = new Map()
  envStore.fetchResources.mockResolvedValue([])
  mockGet.mockResolvedValue(makeEnv())
  mockPut.mockResolvedValue({})
})

describe('EnvironmentDetailPage online agent badge', () => {
  it('shows no badge when no agent is online', async () => {
    mockGet.mockResolvedValue(makeEnv({ agent_status: 'offline', agents_online: 0 }))
    const wrapper = await mountPage()

    // The section itself must be on screen for this absence to mean anything.
    expect(agentsSection(wrapper).find('.agent-row').exists()).toBe(true)
    expect(onlineBadge(wrapper).exists()).toBe(false)
  })

  it('shows the backend count, not a hardcoded 1', async () => {
    // `agent_status` is 'online' whenever any agent is, so an implementation
    // reading it would render "1 online" here.
    mockGet.mockResolvedValue(makeEnv({ agent_status: 'online', agents_online: 2 }))
    const wrapper = await mountPage()

    expect(onlineBadge(wrapper).exists()).toBe(true)
    expect(onlineBadge(wrapper).text()).toBe('2 online')
  })

  it('renders the exact count above one', async () => {
    mockGet.mockResolvedValue(makeEnv({ agent_status: 'online', agents_online: 5 }))
    const wrapper = await mountPage()

    expect(onlineBadge(wrapper).text()).toBe('5 online')
  })

  it('renders no badge for a direct environment', async () => {
    mockGet.mockResolvedValue(makeEnv({ connection_mode: 'direct' }))
    const wrapper = await mountPage()

    expect(wrapper.findAll('.section').some(s => s.find('.section-title').text() === 'Agents')).toBe(false)
  })
})