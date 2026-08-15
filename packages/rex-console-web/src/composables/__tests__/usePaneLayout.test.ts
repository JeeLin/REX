import { describe, it, expect } from 'vitest'
import { usePaneLayout } from '../usePaneLayout'

describe('usePaneLayout', () => {
  it('creates a single-pane root on init', () => {
    const { root, allLeaves } = usePaneLayout()
    expect(root.value.direction).toBe('row')
    expect(allLeaves.value).toHaveLength(1)
    expect(allLeaves.value[0]!.direction).toBeNull()
  })

  it('splits a pane to the right (row direction)', () => {
    const { allLeaves, splitPane, activePaneId } = usePaneLayout()
    const paneId = allLeaves.value[0]!.id
    activePaneId.value = paneId
    splitPane(paneId, 'right')
    // 父节点变为 row，包含两个叶子
    expect(allLeaves.value).toHaveLength(2)
    for (const leaf of allLeaves.value) {
      expect(leaf.size).toBe(50)
    }
  })

  it('splits a pane downward (column direction)', () => {
    const { allLeaves, splitPane, activePaneId } = usePaneLayout()
    const paneId = allLeaves.value[0]!.id
    activePaneId.value = paneId
    splitPane(paneId, 'down')
    expect(allLeaves.value).toHaveLength(2)
  })

  it('wraps in a new container when direction differs from parent', () => {
    const { root, allLeaves, splitPane, applyLayoutPreset, activePaneId } = usePaneLayout()
    // 先用 left-right（row），再在其中一个 pane 下分（down → column 包裹）
    applyLayoutPreset('left-right')
    const firstPane = allLeaves.value[0]!.id
    activePaneId.value = firstPane
    splitPane(firstPane, 'down')
    // 根仍是 row，第一个子节点变 column 包裹两个叶子，第二个保持单叶
    expect(root.value.direction).toBe('row')
    expect(root.value.children).toHaveLength(2)
    expect(allLeaves.value).toHaveLength(3)
  })

  it('supports mixed-direction nesting: vertical then horizontal', () => {
    const { root, allLeaves, splitPane, activePaneId } = usePaneLayout()
    const firstPane = allLeaves.value[0]!.id
    activePaneId.value = firstPane
    // 先上下分屏（down → column 包裹），再在其中一个 pane 下左右分屏（right → row 包裹）
    splitPane(firstPane, 'down')
    const secondPane = allLeaves.value[1]!.id
    activePaneId.value = secondPane
    splitPane(secondPane, 'right')
    // 根方向始终是其初始方向 row，混合方向通过在嵌套层包裹 wrapper 实现
    expect(root.value.direction).toBe('row')
    // 共 3 个叶子
    expect(allLeaves.value).toHaveLength(3)
    // 含 row 与 column 两种方向的容器节点（混合方向嵌套生效）
    const serialized = JSON.stringify(root.value)
    expect(serialized).toContain('"direction":"row"')
    expect(serialized).toContain('"direction":"column"')
  })

  it('closes a pane and merges parent when only one child remains', () => {
    const { allLeaves, splitPane, closePane, activePaneId } = usePaneLayout()
    const firstPane = allLeaves.value[0]!.id
    activePaneId.value = firstPane
    splitPane(firstPane, 'right')
    expect(allLeaves.value).toHaveLength(2)
    const secondPane = allLeaves.value[1]!.id
    closePane(secondPane) // 关闭第二个
    expect(allLeaves.value).toHaveLength(1)
    // 父节点提升，根回到单一叶子
    expect(allLeaves.value[0]!.id).toBe(firstPane)
  })

  it('applies layout presets correctly', () => {
    const { allLeaves, applyLayoutPreset } = usePaneLayout()
    applyLayoutPreset('grid-four')
    expect(allLeaves.value).toHaveLength(4)
    applyLayoutPreset('main-side')
    const leaves = allLeaves.value
    expect(leaves).toHaveLength(2)
    expect(leaves[0]!.size).toBe(70)
    expect(leaves[1]!.size).toBe(30)
    applyLayoutPreset('single')
    expect(allLeaves.value).toHaveLength(1)
  })

  it('serializes and deserializes round-trip', () => {
    const layout = usePaneLayout()
    layout.applyLayoutPreset('grid-four')
    const data = layout.serialize()
    const layout2 = usePaneLayout()
    layout2.deserialize(data)
    expect(layout2.allLeaves.value).toHaveLength(4)
  })

  it('deserialize logs and ignores invalid JSON', () => {
    const { allLeaves, deserialize } = usePaneLayout()
    const before = allLeaves.value.length
    deserialize('not-json{')
    // 无效数据不应破坏现有布局
    expect(allLeaves.value.length).toBe(before)
  })

  it('focusPane updates both activePaneId and lastFocusedPaneId', () => {
    const { allLeaves, splitPane, activePaneId, lastFocusedPaneId, focusPane } = usePaneLayout()
    const firstPane = allLeaves.value[0]!.id
    splitPane(firstPane, 'right')
    const secondPane = allLeaves.value[1]!.id

    // 初始活跃为 split 新生成的 pane
    expect(activePaneId.value).toBe(secondPane)
    // 聚焦第一个 pane：active 与 lastFocused 同步更新
    focusPane(firstPane)
    expect(activePaneId.value).toBe(firstPane)
    expect(lastFocusedPaneId.value).toBe(firstPane)

    // 以聚焦的 pane 为目标再次分屏，应在其旁新增一个 leaf（2 → 3）
    splitPane(lastFocusedPaneId.value, 'right')
    expect(allLeaves.value).toHaveLength(3)
  })

  it('focusPane ignores unknown pane ids', () => {
    const { allLeaves, activePaneId, lastFocusedPaneId, focusPane } = usePaneLayout()
    const firstPane = allLeaves.value[0]!.id
    focusPane('does-not-exist')
    expect(activePaneId.value).toBe(firstPane)
    expect(lastFocusedPaneId.value).toBe(firstPane)
  })

  it('mergeDirection changes root direction', () => {
    const { root, mergeDirection } = usePaneLayout()
    mergeDirection('column')
    expect(root.value.direction).toBe('column')
  })

  it('setPaneTab binds a tab to a leaf', () => {
    const { allLeaves, setPaneTab } = usePaneLayout()
    const first = allLeaves.value[0]!.id
    setPaneTab(first, 'tab-1')
    expect(allLeaves.value[0]!.tabId).toBe('tab-1')
    // 未知 pane 不报错
    setPaneTab('nope', 'tab-2')
  })

  it('serialize includes nodeIdCounter', () => {
    const layout = usePaneLayout()
    layout.splitPane(layout.allLeaves.value[0]!.id, 'right')
    const data = JSON.parse(layout.serialize())
    expect(typeof data.nodeIdCounter).toBe('number')
    expect(data.activePaneId).toBeDefined()
  })

  it('deserialize restores nodeIdCounter', () => {
    const layout = usePaneLayout()
    layout.splitPane(layout.allLeaves.value[0]!.id, 'right')
    const data = layout.serialize()
    const layout2 = usePaneLayout()
    layout2.deserialize(data)
    expect(layout2.allLeaves.value).toHaveLength(2)
  })

  it('left-right and top-bottom presets split correctly', () => {
    const { root, allLeaves, applyLayoutPreset } = usePaneLayout()
    applyLayoutPreset('left-right')
    expect(allLeaves.value).toHaveLength(2)
    expect(allLeaves.value.every((l) => l.size === 50)).toBe(true)
    applyLayoutPreset('top-bottom')
    expect(root.value.direction).toBe('column')
    expect(allLeaves.value).toHaveLength(2)
  })

  it('closePane promotes grandchild when parent collapses to single child', () => {
    const { root, allLeaves, splitPane, closePane, activePaneId } = usePaneLayout()
    const first = allLeaves.value[0]!.id
    activePaneId.value = first
    splitPane(first, 'right')
    // 在第一个 pane 下再分一次（down → 嵌套 column）
    const second = allLeaves.value[1]!.id
    activePaneId.value = second
    splitPane(second, 'down')
    expect(allLeaves.value).toHaveLength(3)
    // 关掉第二个（它所在父容器只剩一个子节点，应被提升）
    closePane(second)
    expect(allLeaves.value).toHaveLength(2)
    expect(root.value.children.length).toBeGreaterThanOrEqual(1)
  })

  it('flatten helper returns depth-sorted leaves', () => {
    const { root, applyLayoutPreset, flatten } = usePaneLayout()
    applyLayoutPreset('grid-four')
    const flat = flatten(root.value, 0)
    expect(flat).toHaveLength(4)
    expect(flat.every((f) => f.direction === null)).toBe(true)
  })

  it('closing the focused pane keeps lastFocusedPaneId valid so split still works', () => {
    const { allLeaves, splitPane, closePane, focusPane, lastFocusedPaneId, activePaneId } = usePaneLayout()
    const first = allLeaves.value[0]!.id
    // 分出新 pane 并聚焦它
    splitPane(first, 'right')
    const second = allLeaves.value[1]!.id
    focusPane(second)
    expect(lastFocusedPaneId.value).toBe(second)
    // 关闭刚聚焦的 pane
    closePane(second)
    // lastFocusedPaneId 不应悬空在已移除的 second 上
    expect(lastFocusedPaneId.value).not.toBe(second)
    // 关闭后以 lastFocusedPaneId 为目标的分栏应仍生效（不再 no-op）
    const target = lastFocusedPaneId.value
    const before = allLeaves.value.length
    splitPane(target, 'right')
    expect(allLeaves.value.length).toBe(before + 1)
    expect(activePaneId.value).not.toBeNull()
  })

  it('applyLayoutPreset keeps lastFocusedPaneId valid after tree rebuild', () => {
    const { allLeaves, splitPane, focusPane, lastFocusedPaneId, applyLayoutPreset } = usePaneLayout()
    const first = allLeaves.value[0]!.id
    splitPane(first, 'right')
    focusPane(allLeaves.value[1]!.id)
    const stale = lastFocusedPaneId.value
    // 重建整棵树，旧 id 全部失效
    applyLayoutPreset('single')
    expect(lastFocusedPaneId.value).not.toBe(stale)
    // 新布局下分栏仍作用于有效 pane（首叶子）
    const target = lastFocusedPaneId.value
    expect(allLeaves.value.find((l) => l.id === target)).toBeDefined()
    const before = allLeaves.value.length
    splitPane(target, 'right')
    expect(allLeaves.value.length).toBe(before + 1)
  })
})
