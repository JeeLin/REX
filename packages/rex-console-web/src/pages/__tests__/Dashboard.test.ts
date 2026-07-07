import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import Dashboard from '../Dashboard.vue'

const i18n = createI18n({
  legacy: false,
  locale: 'zh',
  messages: {
    zh: {
      dashboard: {
        envCount: '环境数',
        resCount: '资源数',
        agentOnline: 'Agent 在线',
        todayOps: '今日操作',
        environments: '环境',
        envCountLabel: '个环境',
        quickConnect: '快速连接',
        quickConnectEmpty: '连接过的资源会出现在这里',
        createEnv: '创建环境',
        loadFailed: '加载失败',
      },
      common: { noData: '暂无数据', retry: '重试' },
      env: {
        title: '环境',
        connectionModeLabel: '直连',
        direct: '直连',
        agentProxy: 'Agent 代理',
        deleteTitle: '删除环境',
        deleteConfirm: '确定删除？',
      },
      status: { online: '在线', offline: '离线' },
      ctx: {
        refreshStats: '刷新统计',
        openDetail: '查看详情',
        newResource: '新建资源',
        addAgent: '添加 Agent',
        openAllWorkspace: '在工作区打开所有',
        editEnv: '编辑环境',
        deleteEnv: '删除环境',
        connect: '连接',
        connectNewTab: '新标签连接',
        copyAddress: '复制地址',
        addFavorite: '收藏',
        removeRecent: '移除最近使用',
      },
    },
  },
})

vi.mock('vue-router', () => ({
  useRouter: () => ({ push: vi.fn() }),
}))

vi.mock('@/composables/useRecent', () => ({
  useRecent: () => ({
    recent: { value: [] },
    removeRecent: vi.fn(),
  }),
}))

vi.mock('@/composables/useSidebar', () => ({
  useSidebar: () => ({ addFavorite: vi.fn() }),
}))

vi.mock('@/composables/useProtocol', () => ({
  useProtocol: () => ({ connectToResource: vi.fn() }),
  getProtocolIcon: () => ({ icon: '?', color: '#888' }),
}))

vi.mock('@/composables/useContextMenu', () => ({
  useContextMenu: () => ({ show: vi.fn() }),
}))

vi.mock('@/api/env', () => ({
  listEnvsWithResources: vi.fn().mockResolvedValue([]),
  deleteEnvironment: vi.fn(),
}))

vi.mock('@/api/audit', () => ({
  getAuditStats: vi.fn().mockResolvedValue({ total: 0 }),
}))

vi.mock('@/api/health', () => ({
  fetchHealth: vi.fn().mockResolvedValue({ connections: { agents_online: 0 } }),
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

describe('Dashboard', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('shows skeleton loader while loading', () => {
    const wrapper = mount(Dashboard, { global: { plugins: [i18n] } })
    expect(wrapper.find('.skeleton-loader').exists()).toBe(true)
  })

  it('renders stat cards after loading', async () => {
    const wrapper = mount(Dashboard, { global: { plugins: [i18n] } })
    await vi.dynamicImportSettled()
    await new Promise(r => setTimeout(r, 10))
    await wrapper.vm.$nextTick()

    expect(wrapper.find('.stats-row').exists()).toBe(true)
    expect(wrapper.findAll('.stat-card').length).toBe(4)
  })

  it('renders section header with environment title', async () => {
    const wrapper = mount(Dashboard, { global: { plugins: [i18n] } })
    await vi.dynamicImportSettled()
    await new Promise(r => setTimeout(r, 10))
    await wrapper.vm.$nextTick()

    expect(wrapper.find('.section-title').text()).toContain('环境')
  })

  it('shows quick connect empty state', async () => {
    const wrapper = mount(Dashboard, { global: { plugins: [i18n] } })
    await vi.dynamicImportSettled()
    await new Promise(r => setTimeout(r, 10))
    await wrapper.vm.$nextTick()

    expect(wrapper.find('.quick-connect-empty').exists()).toBe(true)
  })
})
