import type { Ref } from 'vue'
import type { Tab } from '@/composables/useTabs'

// 工作区分栏渲染所需的共享上下文：由 WorkspacePage 提供，PaneNode / PaneLeaf 注入。
// 这样递归渲染树状分栏时无需层层透传 props。
export interface PaneCtx {
  activePaneId: Ref<string>
  allLeaves: Ref<{ id: string; tabId: string | null }[]>
  // 记录某 pane 被聚焦，统一更新 activePaneId 与 lastFocusedPaneId
  focusPane: (paneId: string) => void
  dragOverPane: Ref<string | null>
  // 分栏/关闭
  splitHorizontal: (paneId?: string) => void
  splitVertical: (paneId?: string) => void
  closePane: (paneId: string) => void
  setPaneTab: (paneId: string, tabId: string | null) => void
  // tab 查询
  findTab: (tabId: string) => Tab | undefined
  activeTabInfo: Ref<Tab | null | undefined>
  // 右键菜单
  onPaneContextMenu: (e: MouseEvent, paneId: string) => void
  // 拖拽
  onPaneDragEnter: (paneId: string) => void
  onPaneDragLeave: (paneId: string) => void
  onPaneDrop: (e: DragEvent, paneId: string) => void
  // 状态更新
  onTabStatusChange: (tabId: string, status: Tab['status']) => void
  onTerminalResize: (cols: number, rows: number) => void
  onEncodingChange: (encoding: string) => void
  // SFTP drawer（全局状态）
  showSftpDrawer: Ref<boolean>
  sftpDrawerHeight: Ref<number>
  toggleSftpDrawer: () => void
  startSftpDrag: (e: MouseEvent) => void
}

export const PANE_CTX = Symbol('paneCtx')
