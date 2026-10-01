import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { ref, computed } from 'vue'
import type { PaneCtx } from '../paneContext'
import { PANE_CTX } from '../paneContext'
import type { Tab } from '@/composables/useTabs'

vi.mock('vue-i18n', () => ({ useI18n: () => ({ t: (k: string) => k }) }))

// Stub heavy child components — they are tested elsewhere.
vi.mock('@/features/terminal/WorkspaceTerminal.vue', () => ({
  default: { template: '<div class="ws-terminal-stub" />', props: ['tabId', 'resourceId', 'name'] },
}))
// __esModule marks the mock as a transpiled module so defineAsyncComponent
// unwraps `default` instead of treating the mock namespace as the component.
// Only defineAsyncComponent targets need this marker; statically imported
// components (e.g. FilesDrawer) are resolved through Vite interop.
vi.mock('@/features/sql/SqlPage.vue', () => ({
  __esModule: true,
  default: { template: '<div class="ws-sql-stub" />', props: ['tabId', 'resourceId', 'dbType'] },
}))
vi.mock('@/features/redis/RedisPage.vue', () => ({
  __esModule: true,
  default: { template: '<div class="ws-redis-stub" />', props: ['resourceId'] },
}))
vi.mock('@/features/files/FilesPage.vue', () => ({
  __esModule: true,
  default: { template: '<div class="ws-files-stub" />', props: ['tabId', 'resourceId', 'protocol'] },
}))
vi.mock('@/features/files/FilesDrawer.vue', () => ({
  default: { template: '<div class="ws-sftp-drawer-stub" />', props: ['resourceId'] },
}))
vi.mock('@/features/sip/SipPage.vue', () => ({
  __esModule: true,
  default: { template: '<div class="ws-sip-stub" />', props: ['resourceId', 'environmentId', 'name'] },
}))

// Import the component after mocks are registered so async imports resolve to stubs.
import PaneLeaf from '../PaneLeaf.vue'
import SqlPageStub from '@/features/sql/SqlPage.vue'
import RedisPageStub from '@/features/redis/RedisPage.vue'
import FilesPageStub from '@/features/files/FilesPage.vue'

function buildCtx(overrides: Partial<PaneCtx> = {}): PaneCtx {
  const leaves = ref([{ id: 'leaf-1', tabId: 'tab-1' }])
  const activePaneId = ref('leaf-1')
  const dragOverPane = ref<string | null>(null)
  const showSftpDrawer = ref(false)
  const sftpDrawerHeight = ref(200)

  const tab: Tab = {
    id: 'tab-1',
    label: 'My SSH Server',
    protocol: 'ssh',
    resourceId: 'res-1',
    status: 'connected',
  }

  return {
    activePaneId,
    allLeaves: leaves,
    focusPane: vi.fn(),
    dragOverPane,
    splitHorizontal: vi.fn(),
    splitVertical: vi.fn(),
    closePane: vi.fn(),
    setPaneTab: vi.fn(),
    findTab: vi.fn(() => tab),
    activeTabInfo: ref(tab),
    onPaneContextMenu: vi.fn(),
    onPaneDragEnter: vi.fn(),
    onPaneDragLeave: vi.fn(),
    onPaneDrop: vi.fn(),
    onTabStatusChange: vi.fn(),
    onTerminalResize: vi.fn(),
    onEncodingChange: vi.fn(),
    showSftpDrawer,
    sftpDrawerHeight,
    toggleSftpDrawer: vi.fn(),
    startSftpDrag: vi.fn(),
    createTab: vi.fn((protocol: string, label: string) => `new-${protocol}-${label}`),
    ...overrides,
  }
}

describe('PaneLeaf', () => {
  let ctx: PaneCtx

  beforeEach(() => {
    ctx = buildCtx()
  })

  // Shared skeleton for the protocol-specific pages (SqlPage / RedisPage / FilesPage):
  // they are all reached through defineAsyncComponent, so the mount has to drain
  // its resolution before any assertion can see the child.
  async function mountPaneWithTab(tab: Tab) {
    ctx = buildCtx({ findTab: vi.fn(() => tab) })
    const wrapper = mount(PaneLeaf, {
      props: { leafId: 'leaf-1' },
      global: { provide: { [PANE_CTX]: ctx } },
    })

    await flushPromises()
    await wrapper.vm.$nextTick()
    return wrapper
  }

  it('renders with the base ws-pane class', () => {
    const wrapper = mount(PaneLeaf, {
      props: { leafId: 'leaf-1' },
      global: { provide: { [PANE_CTX]: ctx } },
    })
    expect(wrapper.find('.ws-pane').exists()).toBe(true)
  })

  it('applies ws-pane--active class when leaf matches activePaneId', () => {
    ctx.activePaneId.value = 'leaf-1'
    const wrapper = mount(PaneLeaf, {
      props: { leafId: 'leaf-1' },
      global: { provide: { [PANE_CTX]: ctx } },
    })
    expect(wrapper.find('.ws-pane--active').exists()).toBe(true)
  })

  it('does not apply ws-pane--active class for a non-active leaf', () => {
    ctx.activePaneId.value = 'other-pane'
    const wrapper = mount(PaneLeaf, {
      props: { leafId: 'leaf-1' },
      global: { provide: { [PANE_CTX]: ctx } },
    })
    expect(wrapper.find('.ws-pane--active').exists()).toBe(false)
  })

  it('displays the tab label in the header', () => {
    const wrapper = mount(PaneLeaf, {
      props: { leafId: 'leaf-1' },
      global: { provide: { [PANE_CTX]: ctx } },
    })
    expect(wrapper.find('.ws-pane-header span').text()).toBe('My SSH Server')
  })

  it('calls focusPane on click', async () => {
    const wrapper = mount(PaneLeaf, {
      props: { leafId: 'leaf-1' },
      global: { provide: { [PANE_CTX]: ctx } },
    })
    await wrapper.find('.ws-pane').trigger('click')
    expect(ctx.focusPane).toHaveBeenCalledWith('leaf-1')
  })

  it('passes the leaf tab id to SqlPage as tabId', async () => {
    const sqlTab: Tab = {
      id: 'tab-1',
      label: 'Analytics',
      protocol: 'sql',
      resourceId: 'res-sql',
      status: 'connected',
    }
    const wrapper = await mountPaneWithTab(sqlTab)

    const sqlPage = wrapper.findComponent(SqlPageStub)
    expect(sqlPage.exists()).toBe(true)
    expect(sqlPage.props('tabId')).toBe('tab-1')
  })

  it('passes the tab resourceId to RedisPage', async () => {
    const redisTab: Tab = {
      id: 'tab-1',
      label: 'Cache',
      protocol: 'redis',
      resourceId: 'res-redis',
      status: 'connected',
    }
    const wrapper = await mountPaneWithTab(redisTab)

    const redisPage = wrapper.findComponent(RedisPageStub)
    expect(redisPage.exists()).toBe(true)
    expect(redisPage.props('resourceId')).toBe('res-redis')
  })

  it('passes the leaf tab id to FilesPage as tabId', async () => {
    const sftpTab: Tab = {
      id: 'tab-1',
      label: 'Assets',
      protocol: 'sftp',
      resourceId: 'res-sftp',
      status: 'connected',
    }
    const wrapper = await mountPaneWithTab(sftpTab)

    const filesPage = wrapper.findComponent(FilesPageStub)
    expect(filesPage.exists()).toBe(true)
    expect(filesPage.props('tabId')).toBe('tab-1')
  })
})
