import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import Agents from '../Agents.vue'

const i18n = createI18n({
  legacy: false,
  locale: 'zh',
  messages: {
    zh: {
      agent: {
        title: 'Agent 管理',
        noAgents: '暂无 Agent',
        noAgentsHint: '部署 Agent 以代理内网资源连接',
        loadFailed: '加载失败',
        restarted: '重启成功',
        restartFailed: '重启失败',
        restartConfirm: '确定重启 {name}？',
      },
      common: { retry: '重试', confirm: '确定', cancel: '取消' },
      confirm: { title: '确认操作' },
    },
  },
})

vi.mock('vue-router', () => ({
  useRouter: () => ({ push: vi.fn() }),
}))

vi.mock('vue-i18n', async () => {
  const actual = await vi.importActual('vue-i18n')
  return { ...actual }
})

vi.mock('@/api/client', () => ({
  default: {
    get: vi.fn().mockResolvedValue({ data: { data: [] } }),
  },
}))

vi.mock('@/api/update', () => ({
  getUpdateStatus: vi.fn().mockResolvedValue({ current_version: '0.67.0' }),
}))

vi.mock('@/api/agent', () => ({
  restartAgent: vi.fn(),
}))

vi.mock('@/composables/useToast', () => ({
  useToast: () => ({
    success: vi.fn(),
    error: vi.fn(),
  }),
}))

vi.mock('@/components/SkeletonLoader.vue', () => ({
  default: { name: 'SkeletonLoader', template: '<div class="skeleton-loader" />' },
}))

vi.mock('@/components/ErrorState.vue', () => ({
  default: { name: 'ErrorState', template: '<div class="error-state" />' },
}))

vi.mock('@/components/EmptyState.vue', () => ({
  default: { name: 'EmptyState', template: '<div class="empty-state" />' },
}))

vi.mock('@/components/ConfirmDialog.vue', () => ({
  default: { name: 'ConfirmDialog', template: '<div class="confirm-dialog" />' },
}))

vi.mock('@/features/agents/AgentCard.vue', () => ({
  default: { name: 'AgentCard', template: '<div class="agent-card" />', props: ['agent', 'hubVersion'] },
}))

vi.mock('@/features/agents/DeployGuide.vue', () => ({
  default: { name: 'DeployGuide', template: '<div class="deploy-guide" />' },
}))

vi.mock('@/features/agents/AgentConfigModal.vue', () => ({
  default: { name: 'AgentConfigModal', template: '<div class="config-modal" />' },
}))

vi.mock('@/features/agents/AgentLogModal.vue', () => ({
  default: { name: 'AgentLogModal', template: '<div class="log-modal" />' },
}))

vi.mock('@/features/agents/AgentResetTokenModal.vue', () => ({
  default: { name: 'AgentResetTokenModal', template: '<div class="reset-modal" />' },
}))

describe('Agents', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('shows skeleton loader while loading', () => {
    const wrapper = mount(Agents, { global: { plugins: [i18n] } })
    expect(wrapper.find('.skeleton-loader').exists()).toBe(true)
  })

  it('renders section header with title', async () => {
    const wrapper = mount(Agents, { global: { plugins: [i18n] } })
    await vi.dynamicImportSettled()
    await new Promise(r => setTimeout(r, 10))
    await wrapper.vm.$nextTick()

    expect(wrapper.find('.section-title').text()).toContain('Agent 管理')
  })

  it('shows empty state when no agents', async () => {
    const wrapper = mount(Agents, { global: { plugins: [i18n] } })
    await vi.dynamicImportSettled()
    await new Promise(r => setTimeout(r, 10))
    await wrapper.vm.$nextTick()

    expect(wrapper.find('.empty-state').exists()).toBe(true)
  })

  it('renders deploy guide', async () => {
    const wrapper = mount(Agents, { global: { plugins: [i18n] } })
    await vi.dynamicImportSettled()
    await new Promise(r => setTimeout(r, 10))
    await wrapper.vm.$nextTick()

    expect(wrapper.find('.deploy-guide').exists()).toBe(true)
  })
})
