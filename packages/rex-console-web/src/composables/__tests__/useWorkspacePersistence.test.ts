import { describe, it, expect, beforeEach } from 'vitest'
import { ref, type Ref } from 'vue'
import { useWorkspacePersistence } from '../useWorkspacePersistence'

const STORAGE_KEY = 'rex-workspace-state'

type PersistedTab = { id: string; label: string; protocol: string; resourceId?: string; environmentId?: string; status: string }

function makeOpts(tabs: Ref<PersistedTab[]>, leaves: { id: string; tabId: string | null }[] = [{ id: 'leaf-1', tabId: null }]) {
  const allLeaves = ref(leaves)
  const bindings: Record<string, string | null> = {}
  return {
    tabs,
    activeTab: ref<string>(''),
    paneLayoutSerialize: () => '{}',
    paneLayoutDeserialize: () => {},
    allLeaves,
    setPaneTab: (paneId: string, tabId: string | null) => {
      bindings[paneId] = tabId
    },
  }
}

function seedStorage(tabs: any[], activeTabId: string | null = null) {
  localStorage.setItem(
    STORAGE_KEY,
    JSON.stringify({
      version: 2,
      tabs,
      activeTabId,
      paneLayout: '{}',
      timestamp: Date.now(),
    }),
  )
}

describe('useWorkspacePersistence', () => {
  beforeEach(() => {
    localStorage.clear()
  })

  it('restores persisted tabs while respecting leaf count', () => {
    const tabs = ref<PersistedTab[]>([]) as Ref<PersistedTab[]>
    seedStorage([
      { id: 't1', label: 'A', protocol: 'ssh', resourceId: 'r1', status: 'connected' },
      { id: 't2', label: 'B', protocol: 'mysql', resourceId: 'r2', status: 'connected' },
    ])
    const { restore } = useWorkspacePersistence(makeOpts(tabs, [
      { id: 'leaf-1', tabId: null },
      { id: 'leaf-2', tabId: null },
    ]))
    expect(restore()).toBe(true)
    expect(tabs.value).toHaveLength(2)
    expect(tabs.value[0]!.id).toBe('t1')
    expect(tabs.value[0]!.status).toBe('disconnected')
  })

  it('drops persisted tabs beyond leaf count', () => {
    const tabs = ref<PersistedTab[]>([]) as Ref<PersistedTab[]>
    seedStorage([
      { id: 't1', label: 'A', protocol: 'ssh', resourceId: 'r1', status: 'connected' },
      { id: 't2', label: 'B', protocol: 'mysql', resourceId: 'r2', status: 'connected' },
      { id: 't3', label: 'C', protocol: 'redis', resourceId: 'r3', status: 'connected' },
    ])
    const { restore } = useWorkspacePersistence(makeOpts(tabs, [{ id: 'leaf-1', tabId: null }, { id: 'leaf-2', tabId: null }]))
    // 只有 2 个 leaf，第 3 个 tab 无 pane 可绑定，应被丢弃
    expect(restore()).toBe(true)
    expect(tabs.value).toHaveLength(2)
  })

  it('alias writeback: a pending live tab absorbs the persisted old tab of the same resource (restart duplicate bug)', () => {
    // 模拟「openResource 已按 resourceId+protocol 创建一个新 tab」后在 onMounted 执行 restore()。
    // 持久化状态里存在同一资源但 id 是旧值（tab-old）。
    const tabs = ref<PersistedTab[]>([
      { id: 'tab-new', label: '京东云', protocol: 'ssh', resourceId: 'r-jd', environmentId: 'e1', status: 'connecting' },
    ]) as Ref<PersistedTab[]>
    seedStorage([
      { id: 'tab-old', label: '京东云', protocol: 'ssh', resourceId: 'r-jd', status: 'connected' },
    ])
    const { restore } = useWorkspacePersistence(makeOpts(tabs))
    // 不应出现「旧+新」两个相同资源的 tab：
    // 本次会话已开的 tab-new 吸收快照里的 tab-old（别名回写），旧的不再 push。
    expect(restore()).toBe(true)
    expect(tabs.value).toHaveLength(1)
    expect(tabs.value[0]!.id).toBe('tab-new')
  })

  it('keeps multiple persisted instances of the same resource distinct', () => {
    // v0.95.0 子任务 1/4：同一资源分栏多开是合法状态，
    // restore 必须按 tab id 恢复，不能按 (resourceId, protocol) 把它们折叠成一个。
    const tabs = ref<PersistedTab[]>([]) as Ref<PersistedTab[]>
    seedStorage([
      { id: 't1', label: 'A', protocol: 'ssh', resourceId: 'r1', status: 'connected' },
      { id: 't2', label: 'A', protocol: 'ssh', resourceId: 'r1', status: 'connected' },
      { id: 't3', label: 'A', protocol: 'ssh', resourceId: 'r1', status: 'connected' },
    ])
    const { restore } = useWorkspacePersistence(makeOpts(tabs, [
      { id: 'leaf-1', tabId: null },
      { id: 'leaf-2', tabId: null },
      { id: 'leaf-3', tabId: null },
    ]))
    expect(restore()).toBe(true)
    expect(tabs.value).toHaveLength(3)
    expect(tabs.value.map((t) => t.id)).toEqual(['t1', 't2', 't3'])
  })
})
