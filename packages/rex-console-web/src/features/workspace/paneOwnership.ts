import type { PaneCtx } from './paneContext'

// Document-level shortcuts are shared by every mounted instance; split panes
// render one page instance per tab, so only the instance owning the keystroke
// may react (and preventDefault) — otherwise one keypress fires twice, and any
// mounted instance would hijack the page's global shortcut. Falls back to the
// active tab when the focused pane is empty, and always handles when rendered
// outside a pane tree.
export function ownsKeystroke(tabId: string | undefined, paneCtx: PaneCtx | null): boolean {
  if (!tabId || !paneCtx) return true
  const focusedTabId = paneCtx.allLeaves.value.find(l => l.id === paneCtx.activePaneId.value)?.tabId
  if (focusedTabId) return focusedTabId === tabId
  return paneCtx.activeTabInfo.value?.id === tabId
}
