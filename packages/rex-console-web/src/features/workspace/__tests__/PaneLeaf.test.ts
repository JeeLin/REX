import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { ref, computed } from 'vue'
import type { PaneCtx } from '../paneContext'
import { PANE_CTX } from '../paneContext'
import type { Tab } from '@/composables/useTabs'

vi.mock('vue-i18n', () => ({ useI18n: () => ({ t: (k: string) => k }) }))

// Stub heavy child components — they are tested elsewhere.
vi.mock('@/features/terminal/WorkspaceTerminal.vue', () => ({
  default: { template: '<div class="ws-terminal-stub" />', props: ['tabId', 'resourceId', 'name'] },
}))
vi.mock('@/features/sql/SqlPage.vue', () => ({
  default: { template: '<div class="ws-sql-stub" />', props: ['resourceId', 'dbType'] },
}))
vi.mock('@/features/redis/RedisPage.vue', () => ({
  default: { template: '<div class="ws-redis-stub" />', props: ['resourceId'] },
}))
vi.mock('@/features/files/FilesPage.vue', () => ({
  default: { template: '<div class="ws-files-stub" />', props: ['resourceId', 'protocol'] },
}))
vi.mock('@/features/files/FilesDrawer.vue', () => ({
  default: { template: '<div class="ws-sftp-drawer-stub" />', props: ['resourceId'] },
}))
vi.mock('@/features/sip/SipPage.vue', () => ({
  default: { template: '<div class="ws-sip-stub" />', props: ['resourceId', 'environmentId', 'name'] },
}))

// Import the component after mocks are registered so async imports resolve to stubs.
import PaneLeaf from '../PaneLeaf.vue'

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
    ...overrides,
  }
}

describe('PaneLeaf', () => {
  let ctx: PaneCtx

  beforeEach(() => {
    ctx = buildCtx()
  })

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
})
