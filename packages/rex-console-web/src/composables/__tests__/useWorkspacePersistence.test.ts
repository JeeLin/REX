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

  it('dedups persisted tabs against already-open tabs by resourceId+protocol (restart duplicate bug)', () => {
    // 模拟「openResource 已按 resourceId+protocol 创建一个新 tab」后在 onMounted 执行 restore()
    const tabs = ref<PersistedTab[]>([
      { id: 'tab-new', label: '京东云', protocol: 'ssh', resourceId: 'r-jd', environmentId: 'e1', status: 'connecting' },
    ]) as Ref<PersistedTab[]>
    // 持久化状态里存在同一资源（resourceId 相同，protocol 相同）但 id 是旧值
    seedStorage([
      { id: 'tab-old', label: '京东云', protocol: 'ssh', resourceId: 'r-jd', status: 'connected' },
    ])
    const { restore } = useWorkspacePersistence(makeOpts(tabs))
    // 不应出现「旧+新」两个相同资源的 tab
    expect(restore()).toBe(true)
    expect(tabs.value).toHaveLength(1)
    expect(tabs.value[0]!.id).toBe('tab-new')
  })
})
