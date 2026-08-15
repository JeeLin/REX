import { ref, computed } from 'vue'

// ========== 数据结构 ==========

export interface PaneNode {
  id: string
  direction: 'row' | 'column' | null  // null = 叶子节点（内容面板）
  children: PaneNode[]
  size: number  // 父容器内占比 (0-100)
  tabId: string | null  // 叶子节点绑定的 tab id
}

// ========== 工具函数 ==========

let nodeIdCounter = 0
function newNode(direction: 'row' | 'column' | null, tabId: string | null = null, size = 50): PaneNode {
  return {
    id: `pane-${++nodeIdCounter}`,
    direction,
    children: [],
    size,
    tabId,
  }
}

// 从 id 查找节点及其父节点
function findNode(node: PaneNode, id: string, parent: PaneNode | null = null): { node: PaneNode; parent: PaneNode | null; index: number } | null {
  if (node.id === id) {
    return { node, parent, index: parent ? parent.children.indexOf(node) : -1 }
  }
  for (const child of node.children) {
    const found = findNode(child, id, node)
    if (found) return found
  }
  return null
}

// 查找叶子节点
function findLeaves(root: PaneNode): PaneNode[] {
  if (root.direction === null) return [root]
  const leaves: PaneNode[] = []
  for (const child of root.children) {
    leaves.push(...findLeaves(child))
  }
  return leaves
}

// 获取所有 pane 的平铺列表（用于渲染）
export interface FlatPane {
  id: string
  depth: number
  direction: 'row' | 'column' | null
  size: number
  tabId: string | null
}

function flatten(root: PaneNode, depth = 0): FlatPane[] {
  if (root.direction === null) {
    return [{ id: root.id, depth, direction: null, size: root.size, tabId: root.tabId }]
  }
  const result: FlatPane[] = []
  for (const child of root.children) {
    result.push(...flatten(child, depth + 1))
  }
  return result
}

// 计算嵌套层级的最大深度
function maxDepth(root: PaneNode): number {
  if (root.direction === null) return 0
  let max = 0
  for (const child of root.children) {
    max = Math.max(max, maxDepth(child) + 1)
  }
  return max
}

// ========== 主 composable ==========

export function usePaneLayout() {
  // 根节点：始终是一个方向容器，包含若干子 pane
  const root = ref<PaneNode>({
    id: 'root',
    direction: 'row',
    children: [newNode(null, null, 100)],
    size: 100,
    tabId: null,
  })

  const activePaneId = ref<string>(root.value.children[0]!.id)

  // 最近一次聚焦（focusin / pointerdown）的 pane。
  // 状态栏分栏按钮、快捷键（Ctrl+\）不带参时优先用它作为目标，
  // 使分栏作用于「用户正在交互的 pane」而非陈旧/默认的 activePaneId。
  const lastFocusedPaneId = ref<string>(activePaneId.value)

  // 所有叶子节点
  const allLeaves = computed(() => findLeaves(root.value))

  // 活跃 pane 信息
  const activePane = computed(() => {
    const result = findNode(root.value, activePaneId.value)
    return result?.node ?? null
  })

  // ========== 操作 ==========

  /**
   * 在指定 pane 旁边分出新 pane
   * @param paneId 目标 pane id
   * @param direction 'right' | 'down' 新 pane 出现在哪个方向
   */
  function splitPane(paneId: string, direction: 'right' | 'down') {
    const result = findNode(root.value, paneId)
    if (!result || !result.parent || result.node.direction !== null) return  // 只能分非根叶子
    if (result.parent.children.length === 0) return  // 无兄弟节点无法插入

    const { node, parent } = result
    if (!parent) return

    const newPane = newNode(null, null, 50)
    const splitDir: 'row' | 'column' = direction === 'right' ? 'row' : 'column'

    // 如果父节点方向一致，直接在父节点的 children 中插入
    if (parent.direction === splitDir) {
      const idx = parent.children.indexOf(node)
      parent.children.splice(idx + 1, 0, newPane)
      // 重新分配大小：所有节点平均分配
      const avg = 100 / parent.children.length
      for (const child of parent.children) {
        child.size = avg
      }
    } else {
      // 方向不同，需要包裹一层
      const wrapper = newNode(splitDir, null, node.size)
      wrapper.children = [node, newPane]
      node.size = 50
      newPane.size = 50

      // 替换 parent.children 中的 node
      const idx = parent.children.indexOf(node)
      parent.children[idx] = wrapper
    }

    activePaneId.value = newPane.id
  }

  /**
   * 关闭指定 pane
   */
  function closePane(paneId: string) {
    const result = findNode(root.value, paneId)
    if (!result || !result.parent) return  // 不能关闭根节点

    const { node, parent, index } = result

    // 从父节点中移除
    parent.children.splice(index, 1)

    // 如果父节点只剩 1 个子节点，提升该子节点替换父节点
    if (parent.children.length === 1 && parent.id !== 'root') {
      const onlyChild = parent.children[0]!
      const grandparentResult = findNode(root.value, parent.id)
      if (grandparentResult?.parent) {
        const gpIdx = grandparentResult.parent.children.indexOf(parent)
        grandparentResult.parent.children[gpIdx] = onlyChild
        onlyChild.size = parent.size
      }
    }

    // 如果根节点只剩 1 个子节点且根节点方向是 row/column，保持不变
    // （根节点始终存在）

    // 更新活跃 pane
    if (activePaneId.value === paneId) {
      const leaves = findLeaves(root.value)
      if (leaves.length > 0) {
        activePaneId.value = leaves[0]!.id
      }
    }
    // lastFocusedPaneId 必须跟随有效 pane：被关闭的 pane 可能正是上次聚焦者，
    // 悬空会令 splitHorizontal/Vetical 以已移除 id 调 splitPane 而 no-op。
    if (!findNode(root.value, lastFocusedPaneId.value)) {
      lastFocusedPaneId.value = activePaneId.value
    }
  }

  /**
   * 合并方向：将两个相邻 pane 的方向改为指定方向
   */
  function mergeDirection(direction: 'row' | 'column') {
    root.value.direction = direction
  }

  /**
   * 应用布局预设
   */
  function applyLayoutPreset(preset: 'single' | 'left-right' | 'top-bottom' | 'grid-four' | 'main-side') {
    nodeIdCounter = 0
    switch (preset) {
      case 'single':
        root.value = {
          id: 'root', direction: 'row',
          children: [newNode(null, null, 100)],
          size: 100, tabId: null,
        }
        break
      case 'left-right':
        root.value = {
          id: 'root', direction: 'row',
          children: [newNode(null, null, 50), newNode(null, null, 50)],
          size: 100, tabId: null,
        }
        break
      case 'top-bottom':
        root.value = {
          id: 'root', direction: 'column',
          children: [newNode(null, null, 50), newNode(null, null, 50)],
          size: 100, tabId: null,
        }
        break
      case 'grid-four': {
        const leftRow = newNode('row', null, 50)
        leftRow.children = [newNode(null, null, 50), newNode(null, null, 50)]
        const rightRow = newNode('row', null, 50)
        rightRow.children = [newNode(null, null, 50), newNode(null, null, 50)]
        root.value = {
          id: 'root', direction: 'column',
          children: [leftRow, rightRow],
          size: 100, tabId: null,
        }
        break
      }
      case 'main-side':
        root.value = {
          id: 'root', direction: 'row',
          children: [newNode(null, null, 70), newNode(null, null, 30)],
          size: 100, tabId: null,
        }
        break
    }
    activePaneId.value = root.value.children[0]!.id
    // 重建整棵树后旧 pane id 全部失效，同步 lastFocusedPaneId 避免分栏 no-op。
    lastFocusedPaneId.value = activePaneId.value
  }

  /**
   * 设置 pane 的 tab 绑定
   */
  function setPaneTab(paneId: string, tabId: string | null) {
    const result = findNode(root.value, paneId)
    if (result) result.node.tabId = tabId
  }

  /**
   * 序列化（用于持久化）
   */
  function serialize(): string {
    return JSON.stringify({
      root: root.value,
      activePaneId: activePaneId.value,
      nodeIdCounter,
    })
  }

  /**
   * 反序列化（用于恢复）
   */
  function deserialize(data: string) {
    try {
      const parsed = JSON.parse(data)
      if (parsed.root) root.value = parsed.root
      if (parsed.activePaneId) activePaneId.value = parsed.activePaneId
      if (parsed.nodeIdCounter) nodeIdCounter = parsed.nodeIdCounter
    } catch (err) {
      console.error('Failed to deserialize pane layout:', err)
    }
  }

  /**
   * 记录某 pane 被聚焦，统一更新 activePaneId 与 lastFocusedPaneId。
   * 供 PaneLeaf 的 focusin / pointerdown 调用，使分栏始终作用于当前交互的 pane。
   */
  function focusPane(paneId: string) {
    if (!findNode(root.value, paneId)) return
    activePaneId.value = paneId
    lastFocusedPaneId.value = paneId
  }

  return {
    root,
    activePaneId,
    activePane,
    allLeaves,
    lastFocusedPaneId,
    focusPane,
    splitPane,
    closePane,
    mergeDirection,
    applyLayoutPreset,
    setPaneTab,
    serialize,
    deserialize,
    flatten,
  }
}
