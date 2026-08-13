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
})
