import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import Environments from '../Environments.vue'

const i18n = createI18n({
  legacy: false,
  locale: 'zh',
  messages: {
    zh: {
      env: {
        title: '环境管理',
        create: '创建环境',
        loadFailed: '加载失败',
        deleteTitle: '删除环境',
        deleteConfirm: '确定删除？',
        resources: '个资源',
        connectionModeLabel: '直连',
        direct: '直连',
        agentProxy: 'Agent 代理',
      },
      common: { noData: '暂无数据', retry: '重试' },
      status: { online: '在线', offline: '离线' },
      ctx: {
        openDetail: '查看详情',
        newResource: '新建资源',
        addAgent: '添加 Agent',
        openAllWorkspace: '在工作区打开所有',
        editEnv: '编辑环境',
        deleteEnv: '删除环境',
      },
      dashboard: { createEnv: '创建环境' },
    },
  },
})

vi.mock('vue-router', () => ({
  useRouter: () => ({ push: vi.fn() }),
}))

vi.mock('@/composables/useContextMenu', () => ({
  useContextMenu: () => ({ show: vi.fn() }),
}))

vi.mock('@/composables/useProtocol', () => ({
  useProtocol: () => ({ connectToResource: vi.fn() }),
}))

vi.mock('@/api/env', () => ({
  listResources: vi.fn().mockResolvedValue([]),
  deleteEnvironment: vi.fn(),
}))

vi.mock('@/api/client', () => ({
  default: {
    get: vi.fn().mockResolvedValue({ data: { data: [] } }),
  },
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

vi.mock('@/components/EnvironmentEditModal.vue', () => ({
  default: { name: 'EnvironmentEditModal', template: '<div class="edit-modal" />' },
}))

vi.mock('@/components/ConfirmDialog.vue', () => ({
  default: { name: 'ConfirmDialog', template: '<div class="confirm-dialog" />' },
}))

describe('Environments', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('shows skeleton loader while loading', () => {
    const wrapper = mount(Environments, { global: { plugins: [i18n] } })
    expect(wrapper.find('.skeleton-loader').exists()).toBe(true)
  })

  it('renders section header with title', async () => {
    const wrapper = mount(Environments, { global: { plugins: [i18n] } })
    await vi.dynamicImportSettled()
    await new Promise(r => setTimeout(r, 10))
    await wrapper.vm.$nextTick()

    expect(wrapper.find('.section-title').text()).toContain('环境管理')
  })

  it('shows empty state when no environments', async () => {
    const wrapper = mount(Environments, { global: { plugins: [i18n] } })
    await vi.dynamicImportSettled()
    await new Promise(r => setTimeout(r, 10))
    await wrapper.vm.$nextTick()

    expect(wrapper.find('.empty-state').exists()).toBe(true)
  })

  it('renders create button', async () => {
    const wrapper = mount(Environments, {
      global: {
        plugins: [i18n],
        stubs: { 'router-link': { template: '<a><slot /></a>' } },
      },
    })
    await vi.dynamicImportSettled()
    await new Promise(r => setTimeout(r, 10))
    await wrapper.vm.$nextTick()

    const createBtn = wrapper.findAll('button, a').find(el => el.text().includes('创建环境'))
    expect(createBtn).toBeDefined()
  })
})
