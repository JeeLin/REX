import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount, flushPromises, type VueWrapper } from '@vue/test-utils'
import { setActivePinia, createPinia } from 'pinia'
import type { Environment } from '@/api/environments'
import type { Resource } from '@/api/resources'

// i18n stub: returns the key itself.
vi.mock('vue-i18n', () => ({ useI18n: () => ({ t: (k: string) => k }) }))

// Router stub: only `push` is used (resource click), never triggered here.
vi.mock('vue-router', () => ({
  useRouter: () => ({ push: vi.fn(), replace: vi.fn() }),
}))

vi.mock('@/api/environments', () => ({
  environmentsApi: { list: vi.fn(), create: vi.fn(), update: vi.fn(), delete: vi.fn() },
}))
vi.mock('@/api/resources', () => ({
  resourcesApi: { listByEnv: vi.fn(), create: vi.fn(), update: vi.fn(), delete: vi.fn(), testConnection: vi.fn() },
}))

// WizardModal is a heavy sibling, unrelated to badge rendering.
vi.mock('@/features/resource/WizardModal.vue', () => ({
  default: { name: 'WizardModalStub', template: '<div class="wizard-modal-stub" />' },
}))

import ResourcePanel from '../ResourcePanel.vue'
import { environmentsApi } from '@/api/environments'
import { resourcesApi } from '@/api/resources'
import { useAppStore } from '@/stores/app'

const mockList = vi.mocked(environmentsApi.list)
const mockListByEnv = vi.mocked(resourcesApi.listByEnv)

function makeEnv(connectionMode: string): Environment {
  return {
    id: 'env-1',
    name: 'Prod',
    description: '',
    connection_mode: connectionMode,
    resource_count: 1,
    agent_status: null,
    registration_token: 'tok',
    created_at: '',
    updated_at: '',
  }
}

function makeResource(envId: string): Resource {
  return {
    id: 'r1',
    environment_id: envId,
    name: 'Web-1',
    protocol: 'ssh',
    host: '10.0.0.1',
    port: 22,
    username: 'root',
    config_json: '{}',
    color: null,
    sort_order: 0,
    created_at: '',
    updated_at: '',
  }
}

async function mountPanel(appMode: 'hub' | 'agent', connectionMode: string): Promise<VueWrapper> {
  const env = makeEnv(connectionMode)
  mockList.mockResolvedValue([env])
  mockListByEnv.mockResolvedValue([makeResource(env.id)])
  const appStore = useAppStore()
  appStore.mode = appMode
  const wrapper = mount(ResourcePanel)
  await flushPromises()
  return wrapper
}

describe('ResourcePanel hub-direct badge', () => {
  let wrapper: VueWrapper | null = null

  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  afterEach(() => {
    wrapper?.unmount()
    wrapper = null
  })

  it('renders the hub-direct badge and ◉ marker for a direct env in agent mode', async () => {
    wrapper = await mountPanel('agent', 'direct')

    const badge = wrapper.find('.rp-group-badge')
    expect(badge.exists()).toBe(true)
    expect(badge.text()).toBe('resourcePanel.badgeHubDirect')

    const marker = wrapper.find('.rp-agent-dot--hub')
    expect(marker.exists()).toBe(true)
    expect(marker.text()).toBe('◉')
  })

  it('does not render the badge or ◉ marker for a direct env in hub (non-agent) mode', async () => {
    wrapper = await mountPanel('hub', 'direct')

    expect(wrapper.find('.rp-group-badge').exists()).toBe(false)
    expect(wrapper.find('.rp-agent-dot--hub').exists()).toBe(false)
    // Fallback status dot is still rendered for the resource row.
    expect(wrapper.find('.rp-agent-dot').exists()).toBe(true)
  })

  it('does not render the badge for an agent-mode env even in agent mode', async () => {
    wrapper = await mountPanel('agent', 'agent')

    expect(wrapper.find('.rp-group-badge').exists()).toBe(false)
    expect(wrapper.find('.rp-agent-dot--hub').exists()).toBe(false)
  })
})
