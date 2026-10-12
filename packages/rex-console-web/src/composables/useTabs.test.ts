import { describe, it, expect, vi } from 'vitest'
import { ref } from 'vue'
import { useTabs, type ResourceNode } from './useTabs'

interface Leaf {
  id: string
  tabId: string | null
}

// createTabs 注入 useTabs 的全部依赖。allLeaves 缺省时不传，
// 用来覆盖「可选依赖未注入」的 fail-closed 路径。
function createTabs(initialLeaves: Leaf[] = [{ id: 'p1', tabId: null }]) {
  const activePaneId = ref(initialLeaves[0]?.id ?? '')
  const leaves = ref<Leaf[]>(initialLeaves)
  const focusPane = vi.fn((paneId: string) => {
    activePaneId.value = paneId
  })

  const tabs = useTabs({
    activePaneId,
    setPaneTab: (paneId, tabId) => {
      const leaf = leaves.value.find((l) => l.id === paneId)
      if (leaf) leaf.tabId = tabId
    },
    allLeaves: leaves,
    focusPane,
  })

  return { ...tabs, leaves, activePaneId, focusPane }
}

function sshNode(id = 'res-1'): ResourceNode {
  return { id, name: 'ssh-box', protocol: 'ssh' }
}

describe('useTabs.openResource — 同一资源能否开第二个实例', () => {
  it('活动格空闲时允许同一资源再开一个新实例（分栏场景）', () => {
    // 两格布局：p1 已有 tab，p2 空闲且为活动格
    const t = createTabs([
      { id: 'p1', tabId: 'tab-old' },
      { id: 'p2', tabId: null },
    ])
    t.tabs.value.push({
      id: 'tab-old',
      label: 'ssh-box',
      protocol: 'ssh',
      resourceId: 'res-1',
      status: 'connected',
    })
    t.activePaneId.value = 'p2'

    t.openResource(sshNode())

    expect(t.tabs.value.filter((x) => x.resourceId === 'res-1')).toHaveLength(2)
    expect(t.tabs.value).toHaveLength(2)
    // 新实例必须落在那个空闲格上
    expect(t.leaves.value.find((l) => l.id === 'p2')?.tabId).toBe(t.tabs.value[1]!.id)
    // 原来那格不受影响（one-tab-one-pane）
    expect(t.leaves.value.find((l) => l.id === 'p1')?.tabId).toBe('tab-old')
  })

  it('活动格已有 tab 时走去重，不新开（防双击刷 tab）', () => {
    const t = createTabs()
    t.openResource(sshNode())
    const firstId = t.tabs.value[0]!.id
    t.activePaneId.value = 'p1'

    // 活动格已被第一个实例占着 → 仍应聚焦既有实例
    t.openResource(sshNode())

    expect(t.tabs.value).toHaveLength(1)
    expect(t.tabs.value[0]!.id).toBe(firstId)
  })

  it('未注入 allLeaves 时重复 openResource 不新增 tab（fail-closed）', () => {
    // allLeaves 不传 —— useTabs 无法证明活动格空闲，必须保守去重。
    const activePaneId = ref('p1')
    const t = useTabs({
      activePaneId,
      setPaneTab: vi.fn(),
      focusPane: vi.fn(),
    })

    t.openResource(sshNode())
    t.openResource(sshNode())

    // 这条用例承重的是 fail-closed 语义：还原成 `!!allLeaves?.value…`
    // 的 fail-open 写法后，这里会得到 length 2 而失败。
    expect(t.tabs.value).toHaveLength(1)
  })

  it('不同资源始终各自新建 tab', () => {
    const t = createTabs()
    t.openResource(sshNode('res-1'))
    t.openResource(sshNode('res-2'))

    expect(t.tabs.value).toHaveLength(2)
    expect(t.tabs.value.map((x) => x.resourceId)).toEqual(['res-1', 'res-2'])
  })

  it('协议不同的同资源 id 视为不同 tab', () => {
    const t = createTabs()
    t.openResource(sshNode('res-1'))
    t.openResource({ id: 'res-1', name: 'db', protocol: 'mysql' })

    expect(t.tabs.value).toHaveLength(2)
  })
})