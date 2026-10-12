import { describe, it, expect, beforeEach } from 'vitest'
import { ref, nextTick } from 'vue'
import { useWorkspacePersistence } from './useWorkspacePersistence'

interface Leaf {
  id: string
  tabId: string | null
}

const STORAGE_KEY = 'rex-workspace-state'

function createHarness(initialLeaves: Leaf[] = [{ id: 'p1', tabId: null }]) {
  const tabs = ref<Array<{ id: string; label: string; protocol: string; resourceId?: string; environmentId?: string; status: string }>>([])
  const activeTab = ref('')
  const leaves = ref<Leaf[]>(initialLeaves.map((l) => ({ ...l })))
  let serializedLayout = JSON.stringify({ type: 'root' })

  const api = useWorkspacePersistence({
    tabs,
    activeTab,
    paneLayoutSerialize: () => serializedLayout,
    paneLayoutDeserialize: (data) => {
      const parsed = JSON.parse(data)
      // 模拟布局恢复后叶数变化：由布局决定 leaves 数量
      const count = Number(parsed.leafCount ?? leaves.value.length)
      leaves.value = Array.from({ length: count }, (_, i) => ({
        id: `leaf${i + 1}`,
        tabId: null,
      }))
    },
    allLeaves: leaves,
    setPaneTab: (paneId, tabId) => {
      const leaf = leaves.value.find((l) => l.id === paneId)
      if (leaf) leaf.tabId = tabId
    },
  })

  return {
    ...api,
    tabs,
    activeTab,
    leaves,
    setLayout: (n: number) => {
      serializedLayout = JSON.stringify({ type: 'root', leafCount: n })
    },
  }
}

function writeSnapshot(state: Record<string, unknown>) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify({ timestamp: Date.now(), ...state }))
}

describe('useWorkspacePersistence — 资源组 + 布局恢复', () => {
  beforeEach(() => {
    localStorage.clear()
  })

  it('恢复同资源的多个实例（分栏各开一个）不被折叠成一个', () => {
    writeSnapshot({
      version: 2,
      tabs: [
        { id: 'tab-1', label: 'box', protocol: 'ssh', resourceId: 'res-1', status: 'connected' },
        { id: 'tab-2', label: 'box', protocol: 'ssh', resourceId: 'res-1', status: 'connected' },
        { id: 'tab-3', label: 'box', protocol: 'ssh', resourceId: 'res-1', status: 'connected' },
      ],
      activeTabId: 'tab-2',
      paneLayout: JSON.stringify({ type: 'root', leafCount: 3 }),
    })

    const h = createHarness()
    h.setLayout(3)
    expect(h.restore()).toBe(true)

    // 三个同资源实例必须全部恢复 —— 按 (resourceId, protocol) 去重会只剩 1 个
    expect(h.tabs.value.filter((t) => t.resourceId === 'res-1')).toHaveLength(3)
    expect(h.activeTab.value).toBe('tab-2')
  })

  it('先恢复布局再恢复 tabs：叶数取自恢复后的布局而非默认单格', () => {
    writeSnapshot({
      version: 2,
      tabs: [
        { id: 'tab-1', label: 'a', protocol: 'ssh', resourceId: 'res-a', status: 'connected' },
        { id: 'tab-2', label: 'b', protocol: 'mysql', resourceId: 'res-b', status: 'connected' },
        { id: 'tab-3', label: 'c', protocol: 'redis', resourceId: 'res-c', status: 'connected' },
      ],
      activeTabId: null,
      paneLayout: JSON.stringify({ type: 'root', leafCount: 3 }),
    })

    const h = createHarness()
    h.setLayout(3)
    h.restore()

    // 布局先恢复出 3 叶，tabs 才能恢复 3 个。
    // 顺序反了会先按 leafCount=1 截断，只剩 tab-1。
    expect(h.leaves.value).toHaveLength(3)
    expect(h.tabs.value).toHaveLength(3)
  })

  it('恢复后的 tab 分别绑定到不同叶（保持一 tab 一叶）', () => {
    writeSnapshot({
      version: 2,
      tabs: [
        { id: 'tab-1', label: 'a', protocol: 'ssh', resourceId: 'res-a', status: 'connected' },
        { id: 'tab-2', label: 'b', protocol: 'ssh', resourceId: 'res-a', status: 'connected' },
      ],
      activeTabId: null,
      paneLayout: JSON.stringify({ type: 'root', leafCount: 2 }),
    })

    const h = createHarness()
    h.setLayout(2)
    h.restore()

    const bound = h.leaves.value.map((l) => l.tabId).filter(Boolean)
    expect(new Set(bound).size).toBe(2)
  })

  it('本次会话已打开同 id 的 tab 时不重复 push', () => {
    writeSnapshot({
      version: 2,
      tabs: [{ id: 'tab-live', label: 'box', protocol: 'ssh', resourceId: 'res-1', status: 'connected' }],
      activeTabId: 'tab-live',
      paneLayout: JSON.stringify({ type: 'root', leafCount: 1 }),
    })

    const h = createHarness()
    h.setLayout(1)
    // 模拟 restore 前，本次会话已经先开了同一个 tab
    h.tabs.value.push({
      id: 'tab-live',
      label: 'box',
      protocol: 'ssh',
      resourceId: 'res-1',
      status: 'connecting',
    })

    h.restore()

    // 同一个 id 只能有一条
    expect(h.tabs.value.filter((t) => t.id === 'tab-live')).toHaveLength(1)
    // 本次会话的状态（connecting）不应被快照的 disconnected 覆盖
    expect(h.tabs.value[0]!.status).toBe('connecting')
  })

  it('清掉指向不存在 tab 的悬空绑定', () => {
    writeSnapshot({
      version: 2,
      tabs: [{ id: 'tab-1', label: 'a', protocol: 'ssh', resourceId: 'res-a', status: 'connected' }],
      activeTabId: null,
      paneLayout: JSON.stringify({ type: 'root', leafCount: 1 }),
    })

    const h = createHarness()
    h.setLayout(1)
    // paneLayoutDeserialize 重建 leaves 后，故意留一个指向幽灵 tab 的绑定
    h.restore()
    h.leaves.value[0]!.tabId = 'tab-ghost'

    // 再次 restore 时该悬空绑定应被清掉
    h.restore()
    expect(h.leaves.value[0]!.tabId).not.toBe('tab-ghost')
  })

  it('纯布局操作（不改 tabs / activeTab）也落盘', async () => {
    const h = createHarness()
    h.setLayout(2)
    h.tabs.value.push({ id: 'tab-1', label: 'a', protocol: 'ssh', resourceId: 'res-a', status: 'connected' })
    h.activeTab.value = 'tab-1'
    await nextTick()

    localStorage.clear()

    // 只改布局：split 出一个新叶，tabs 与 activeTab 都不动
    h.leaves.value.push({ id: 'leaf2', tabId: null })
    await nextTick()

    const raw = localStorage.getItem(STORAGE_KEY)
    // watch 未覆盖 allLeaves 时这里会是 null —— 纯布局操作丢失
    expect(raw).not.toBeNull()
    expect(JSON.parse(raw!).tabs).toHaveLength(1)
  })

  it('快照超过 24h 过期则不恢复并清掉快照', () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        version: 2,
        tabs: [{ id: 'tab-1', label: 'a', protocol: 'ssh', resourceId: 'res-a', status: 'connected' }],
        activeTabId: 'tab-1',
        paneLayout: JSON.stringify({ type: 'root', leafCount: 1 }),
        timestamp: Date.now() - 25 * 60 * 60 * 1000,
      }),
    )

    const h = createHarness()
    h.setLayout(1)
    expect(h.restore()).toBe(false)
    expect(localStorage.getItem(STORAGE_KEY)).toBeNull()
  })
})