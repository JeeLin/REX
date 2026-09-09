/**
 * 统一右键菜单系统
 * 参考 DBX buildContextMenu + menu descriptors
 */

import { formatShortcut } from './platform'

// ── 类型定义 ──────────────────────────────────────────────

export interface MenuItem {
  /** 显示标签 */
  label: string
  /** 图标（emoji 或 icon name） */
  icon?: string
  /** 快捷键（显示在右侧） */
  shortcut?: string
  /** 执行的动作 */
  action?: () => void
  /** 危险操作（标红） */
  danger?: boolean
  /** 禁用状态（灰显） */
  disabled?: boolean
  /** 分隔线 */
  separator?: boolean
  /** 子菜单 */
  children?: MenuItem[]
}

export type NodeType = 'connection' | 'database' | 'table' | 'column' |
  'file' | 'folder' | 'redis-key' | 'ssh-session' | 'tab'

export interface MenuContext {
  type: NodeType
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  data: Record<string, any>
}

// ── 菜单定义 ──────────────────────────────────────────────

const menuDefinitions: Record<NodeType, (ctx: MenuContext) => MenuItem[]> = {
  connection: (ctx) => [
    { label: 'Connect', icon: '🔗', action: () => ctx.data.connect?.() },
    { label: 'Edit', icon: '✏️', shortcut: formatShortcut('Mod+E'), action: () => ctx.data.edit?.() },
    { separator: true, label: '' },
    { label: 'Duplicate', icon: '📋', action: () => ctx.data.duplicate?.() },
    { label: 'Export', icon: '📤', children: [
      { label: 'As JSON', action: () => ctx.data.export?.('json') },
      { label: 'As YAML', action: () => ctx.data.export?.('yaml') },
    ]},
    { separator: true, label: '' },
    { label: 'Delete', icon: '🗑️', danger: true, action: () => ctx.data.delete?.() },
  ],

  database: (ctx) => [
    { label: 'Open', icon: '📂', action: () => ctx.data.open?.() },
    { label: 'New Table', icon: '➕', action: () => ctx.data.newTable?.() },
    { separator: true, label: '' },
    { label: 'Export', icon: '📤', children: [
      { label: 'As SQL', action: () => ctx.data.export?.('sql') },
      { label: 'As CSV', action: () => ctx.data.export?.('csv') },
    ]},
    { separator: true, label: '' },
    { label: 'Drop Database', icon: '⚠️', danger: true, action: () => ctx.data.drop?.() },
  ],

  table: (ctx) => [
    { label: 'Open Table', icon: '📊', shortcut: formatShortcut('Enter'), action: () => ctx.data.open?.() },
    { label: 'Design Table', icon: '🔧', action: () => ctx.data.design?.() },
    { separator: true, label: '' },
    { label: 'Copy Name', icon: '📋', shortcut: formatShortcut('Mod+C'), action: () => ctx.data.copyName?.() },
    { label: 'Truncate', icon: '🧹', danger: true, action: () => ctx.data.truncate?.() },
    { label: 'Drop Table', icon: '⚠️', danger: true, action: () => ctx.data.drop?.() },
  ],

  column: (ctx) => [
    { label: 'Copy Name', icon: '📋', action: () => ctx.data.copyName?.() },
    { label: 'Copy Value', icon: '📋', action: () => ctx.data.copyValue?.() },
    { separator: true, label: '' },
    { label: 'Edit Column', icon: '✏️', action: () => ctx.data.edit?.() },
  ],

  file: (ctx) => [
    { label: 'Open', icon: '📄', action: () => ctx.data.open?.() },
    { label: 'Download', icon: '⬇️', shortcut: formatShortcut('Mod+S'), action: () => ctx.data.download?.() },
    { separator: true, label: '' },
    { label: 'Rename', icon: '✏️', shortcut: 'F2', action: () => ctx.data.rename?.() },
    { label: 'Copy', icon: '📋', shortcut: formatShortcut('Mod+C'), action: () => ctx.data.copy?.() },
    { label: 'Cut', icon: '✂️', shortcut: formatShortcut('Mod+X'), action: () => ctx.data.cut?.() },
    { separator: true, label: '' },
    { label: 'Delete', icon: '🗑️', danger: true, shortcut: 'Del', action: () => ctx.data.delete?.() },
  ],

  folder: (ctx) => [
    { label: 'Open', icon: '📂', action: () => ctx.data.open?.() },
    { label: 'Upload Here', icon: '⬆️', action: () => ctx.data.upload?.() },
    { separator: true, label: '' },
    { label: 'New File', icon: '📄', action: () => ctx.data.newFile?.() },
    { label: 'New Folder', icon: '📁', action: () => ctx.data.newFolder?.() },
    { separator: true, label: '' },
    { label: 'Rename', icon: '✏️', shortcut: 'F2', action: () => ctx.data.rename?.() },
    { label: 'Delete', icon: '🗑️', danger: true, shortcut: 'Del', action: () => ctx.data.delete?.() },
  ],

  'redis-key': (ctx) => [
    { label: 'View Value', icon: '👁️', action: () => ctx.data.view?.() },
    { label: 'Copy Key', icon: '📋', action: () => ctx.data.copyKey?.() },
    { label: 'Copy Value', icon: '📋', action: () => ctx.data.copyValue?.() },
    { separator: true, label: '' },
    { label: 'Set TTL', icon: '⏱️', action: () => ctx.data.setTTL?.() },
    { label: 'Delete Key', icon: '🗑️', danger: true, action: () => ctx.data.delete?.() },
  ],

  'ssh-session': (ctx) => [
    { label: 'Disconnect', icon: '🔌', action: () => ctx.data.disconnect?.() },
    { separator: true, label: '' },
    { label: 'Copy Command', icon: '📋', action: () => ctx.data.copyCommand?.() },
  ],

  tab: (ctx) => [
    { label: 'Close', icon: '❌', shortcut: formatShortcut('Mod+W'), action: () => ctx.data.close?.() },
    { label: 'Close Others', icon: '❌', action: () => ctx.data.closeOthers?.() },
    { label: 'Close to Right', icon: '❌', action: () => ctx.data.closeRight?.() },
    { separator: true, label: '' },
    { label: 'Copy Tab', icon: '📋', action: () => ctx.data.copyTab?.() },
    { label: 'Move to Window', icon: '🪟', action: () => ctx.data.moveToWindow?.() },
  ],
}

// ── 公开 API ──────────────────────────────────────────────

/**
 * 根据节点类型构建右键菜单
 */
export function buildContextMenu(type: NodeType, context: MenuContext): MenuItem[] {
  const builder = menuDefinitions[type]
  if (!builder) return []
  return builder(context)
}

/**
 * Vue 指令式右键菜单（简化版）
 * 实际项目可封装为 v-context-menu 指令
 */
export function showContextMenu(
  event: MouseEvent,
  items: MenuItem[],
  options?: { onSelect?: (item: MenuItem) => void }
): void {
  // 移除已有的菜单
  const existing = document.querySelector('.ctx-menu')
  if (existing) existing.remove()

  // 创建菜单容器
  const menu = document.createElement('div')
  menu.className = 'ctx-menu'
  menu.style.cssText = `
    position: fixed;
    left: ${event.clientX}px;
    top: ${event.clientY}px;
    z-index: 10000;
    min-width: 180px;
    background: var(--color-bg-elevated, #1e1e2e);
    border: 1px solid var(--color-border, #333);
    border-radius: 8px;
    box-shadow: 0 8px 32px rgba(0,0,0,0.4);
    padding: 4px 0;
    font-size: 13px;
  `

  // 渲染菜单项
  items.forEach(item => {
    if (item.separator) {
      const sep = document.createElement('div')
      sep.style.cssText = 'height:1px;background:var(--color-border,#333);margin:4px 0;'
      menu.appendChild(sep)
      return
    }

    const el = document.createElement('div')
    el.style.cssText = `
      display: flex;
      align-items: center;
      padding: 6px 12px;
      gap: 8px;
      cursor: pointer;
      color: ${item.danger ? '#ef4444' : 'var(--color-text, #e0e0e0)'};
      opacity: ${item.disabled ? '0.4' : '1'};
      pointer-events: ${item.disabled ? 'none' : 'auto'};
    `
    el.innerHTML = `
      <span>${item.icon || ''}</span>
      <span style="flex:1">${item.label}</span>
      ${item.shortcut ? `<span style="font-size:11px;color:var(--color-text-muted,#888);font-family:monospace">${item.shortcut}</span>` : ''}
    `
    el.addEventListener('mouseenter', () => {
      el.style.background = 'var(--color-bg-hover, #2a2a3a)'
    })
    el.addEventListener('mouseleave', () => {
      el.style.background = 'transparent'
    })
    el.addEventListener('click', () => {
      menu.remove()
      if (item.action) {
        if (item.danger) {
          // 危险操作需要二次确认
          if (confirm(`Are you sure you want to ${item.label.toLowerCase()}?`)) {
            item.action()
          }
        } else {
          item.action()
        }
      }
      options?.onSelect?.(item)
    })
    menu.appendChild(el)
  })

  document.body.appendChild(menu)

  // 点击外部关闭
  const close = (e: MouseEvent) => {
    if (!menu.contains(e.target as Node)) {
      menu.remove()
      document.removeEventListener('click', close)
    }
  }
  setTimeout(() => document.addEventListener('click', close), 0)
}
