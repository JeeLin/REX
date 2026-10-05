import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { nextTick } from 'vue'
import { mount, flushPromises, enableAutoUnmount } from '@vue/test-utils'
import FilesPage from '../FilesPage.vue'

// 组件通过 vue-i18n 的 t() 取文案，测试按点分路径解析真实 locale 文件后断言。
vi.mock('vue-i18n', async () => {
  const locale = (await import('@/i18n/locales/zh.json')).default as unknown as Record<string, unknown>
  return {
    useI18n: () => ({
      t: (key: string, fallback?: string) => {
        let node: unknown = locale
        for (const part of key.split('.')) {
          if (node === null || typeof node !== 'object') return fallback ?? key
          node = (node as Record<string, unknown>)[part]
        }
        return typeof node === 'string' ? node : (fallback ?? key)
      },
    }),
  }
})

const { mockConnect, mockDisconnect, mockListFiles, mockGetCapability } = vi.hoisted(() => ({
  mockConnect: vi.fn(),
  mockDisconnect: vi.fn(),
  mockListFiles: vi.fn(),
  mockGetCapability: vi.fn(),
}))

const { storeStub } = vi.hoisted(() => ({
  storeStub: {
    tasks: undefined as unknown as Map<string, unknown>,
    pushBrowserTask: vi.fn(),
    updateBrowserTask: vi.fn(),
    move: vi.fn(),
    copy: vi.fn(),
    trackSync: vi.fn(),
    monitor: vi.fn(),
    cancel: vi.fn(),
    dismissCompleted: vi.fn(),
    connectWs: vi.fn(),
    reconnectWs: vi.fn(),
    disconnectWs: vi.fn(),
  },
}))

vi.mock('@/api/files', () => ({
  connect: (...args: unknown[]) => mockConnect(...args),
  disconnect: (...args: unknown[]) => mockDisconnect(...args),
  listFiles: (...args: unknown[]) => mockListFiles(...args),
  getCapability: (...args: unknown[]) => mockGetCapability(...args),
}))

// reactive Map 会被 useTransfer 的 computed / useFiles 的 watch 追踪：队列行渲染与
// 「同步终态后自动刷新面板」都能在测试里被观测到。
vi.mock('@/stores/transfer', async () => {
  const { reactive } = await import('vue')
  storeStub.tasks = reactive(new Map()) as unknown as Map<string, unknown>
  return { useTransferStore: () => storeStub }
})

vi.mock('@/components/ui/Button.vue', () => ({
  default: {
    template: '<button><slot /></button>',
    props: ['variant', 'icon', 'size', 'disabled'],
  },
}))

vi.mock('../MobileFilesBar.vue', () => ({
  default: { template: '<div class="mobile-files-bar-stub" />', props: ['selectedCount'] },
}))

vi.mock('../FileEditorDialog.vue', () => ({
  default: { template: '<div class="file-editor-dialog-stub" />', props: ['visible', 'sessionId', 'filePath', 'protocol'] },
}))

// 对话框 stub：点击即 emit created(taskId)，只驱动「创建后登记队列」这一段接线。
vi.mock('../FolderSyncDialog.vue', () => ({
  default: {
    template: '<button class="fsd-stub" @click="$emit(\'created\', \'sync-9\')">sync</button>',
    props: ['open', 'source', 'target'],
    emits: ['close', 'created', 'error'],
  },
}))

const SYNC_SOURCE = { resource_id: 'res-1', path: '/src/dir/' }
const SYNC_TARGET = { resource_id: 'res-1', path: '/dst/dir/' }

function syncItem(over: Record<string, unknown> = {}) {
  return {
    id: 'sync-9',
    kind: 'server',
    direction: 'copy',
    name: 'dir',
    size: 1000,
    transferred: 400,
    progress: 40,
    speed: 0,
    status: 'running',
    eta: null,
    source_path: SYNC_SOURCE.path,
    target_path: SYNC_TARGET.path,
    task_id: 'sync-9',
    task_kind: 'sync',
    phase: 'running',
    error: null,
    ...over,
  }
}

async function mountPage() {
  const w = mount(FilesPage, {
    props: { resourceId: 'res-1', protocol: 'sftp' },
    global: { stubs: { Teleport: true } },
  })
  await flushPromises()
  return w
}

/** 让 useFiles 拿到非空端点（等价于右键菜单里打开对话框后的状态）。 */
function setEndpoints(w: Awaited<ReturnType<typeof mountPage>>) {
  ;(w.vm as unknown as Record<string, unknown>).syncSource = SYNC_SOURCE
  ;(w.vm as unknown as Record<string, unknown>).syncTarget = SYNC_TARGET
}

// 每个用例卸载自己挂载的页面：未卸载的实例会留下 watch，导致共享 store 上的
// 终态事件被重复计入面板刷新次数。
enableAutoUnmount(afterEach)

beforeEach(() => {
  setActivePinia(createPinia())
  vi.clearAllMocks()
  storeStub.tasks.clear()
  mockConnect.mockResolvedValue('test-session-123')
  mockListFiles.mockResolvedValue([])
  mockGetCapability.mockResolvedValue({ capabilities: {} })
})

describe('FilesPage — folder sync queue row (v0.92.0 子任务 5)', () => {
  it('registers the created sync task in the transfer store with both endpoints', async () => {
    const w = await mountPage()
    setEndpoints(w)

    await w.find('.fsd-stub').trigger('click')

    expect(storeStub.trackSync).toHaveBeenCalledWith('sync-9', SYNC_SOURCE, SYNC_TARGET)
  })

  it('shows the sync row with source → target, phase word, progress and cancel', async () => {
    storeStub.tasks.set('sync-9', syncItem())
    const w = await mountPage()
    ;(w.vm as unknown as { transfer: { showTransferQueue: boolean } }).transfer.showTransferQueue = true
    await nextTick()

    expect(w.find('.tq-item-type').text()).toBe('🔁')
    expect(w.find('.tq-item-path').text()).toBe('/src/dir/ → /dst/dir/')
    expect(w.find('.tq-item-phase').text()).toBe('同步中')
    expect(w.find('.tq-item-pct').text()).toBe('40%')

    await w.find('.tq-btn--cancel').trigger('click')
    expect(storeStub.cancel).toHaveBeenCalledWith('sync-9')
  })

  it('names each sync phase while the row keeps the in-flight state', async () => {
    storeStub.tasks.set('sync-9', syncItem({ phase: 'scanning', transferred: 0, progress: 0 }))
    const w = await mountPage()
    ;(w.vm as unknown as { transfer: { showTransferQueue: boolean } }).transfer.showTransferQueue = true
    await nextTick()

    expect(w.find('.tq-item-phase').text()).toBe('扫描中')

    const task = storeStub.tasks.get('sync-9') as { phase: string }
    task.phase = 'planning'
    await nextTick()
    expect(w.find('.tq-item-phase').text()).toBe('规划中')

    task.phase = 'verifying'
    await nextTick()
    expect(w.find('.tq-item-phase').text()).toBe('校验中')
  })

  it('reloads both panels once a tracked sync task reaches a terminal state', async () => {
    const w = await mountPage()
    setEndpoints(w)
    await w.find('.fsd-stub').trigger('click')
    const before = mockListFiles.mock.calls.length

    storeStub.tasks.set('sync-9', syncItem({ status: 'running' }))
    await nextTick()
    expect(mockListFiles.mock.calls.length).toBe(before)

    const task = storeStub.tasks.get('sync-9') as { status: string; phase: string; progress: number }
    task.status = 'done'
    task.phase = 'completed'
    task.progress = 100
    await nextTick()

    expect(mockListFiles.mock.calls.length).toBe(before + 2)
  })
})
