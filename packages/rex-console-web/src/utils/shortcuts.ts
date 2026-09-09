/**
 * 集中式键盘快捷键注册表
 * 参考 DBX shortcutRegistry 模式
 */

import { ref, onMounted, onBeforeUnmount } from 'vue'
import { modKey } from './platform'

// ── 类型定义 ──────────────────────────────────────────────

export type ShortcutScope = 'global' | 'terminal' | 'editor' | 'files' | 'redis'

export interface ShortcutDef {
  /** 唯一标识 */
  id: string
  /** 按键组合，用 + 分隔，Mod 表示平台修饰键 (Cmd/Ctrl) */
  keys: string
  /** 作用域 */
  scope: ShortcutScope
  /** 执行的动作 */
  action: () => void
  /** 显示标签（用户可见） */
  label: string
  /** 分类（用于 Quick Open 分组） */
  category: string
  /** 是否在输入框内也生效（默认 false） */
  allowInInput?: boolean
  /** 是否启用（默认 true） */
  enabled?: boolean
}

// ── 注册表 ────────────────────────────────────────────────

const registry = new Map<string, ShortcutDef>()

/**
 * 注册快捷键
 */
export function registerShortcut(def: ShortcutDef): void {
  registry.set(def.id, { ...def, enabled: def.enabled ?? true })
}

/**
 * 注销快捷键
 */
export function unregisterShortcut(id: string): void {
  registry.delete(id)
}

/**
 * 按作用域获取快捷键
 */
export function getShortcutsByScope(scope: ShortcutScope): ShortcutDef[] {
  return Array.from(registry.values()).filter(
    s => s.scope === scope && s.enabled !== false
  )
}

/**
 * 获取所有快捷键
 */
export function getAllShortcuts(): ShortcutDef[] {
  return Array.from(registry.values()).filter(s => s.enabled !== false)
}

// ── 按键解析 ──────────────────────────────────────────────

interface ParsedKey {
  ctrl: boolean
  meta: boolean
  shift: boolean
  alt: boolean
  key: string
}

function parseKeyCombo(keys: string): ParsedKey {
  const parts = keys.toLowerCase().split('+').map(s => s.trim())
  const mod = modKey()
  return {
    ctrl: parts.includes('ctrl') || (mod === 'Control' && parts.includes('mod')),
    meta: parts.includes('meta') || (mod === 'Meta' && parts.includes('mod')),
    shift: parts.includes('shift'),
    alt: parts.includes('alt'),
    key: parts.find(p => !['ctrl', 'meta', 'shift', 'alt', 'mod'].includes(p)) || '',
  }
}

function matchKeyEvent(e: KeyboardEvent, parsed: ParsedKey): boolean {
  const ctrlMatch = parsed.ctrl ? (e.ctrlKey || e.metaKey) : !e.ctrlKey
  const metaMatch = parsed.meta ? (e.metaKey || e.ctrlKey) : !e.metaKey
  const shiftMatch = parsed.shift ? e.shiftKey : !e.shiftKey
  const altMatch = parsed.alt ? e.altKey : !e.altKey
  const keyMatch = e.key.toLowerCase() === parsed.key

  return ctrlMatch && metaMatch && shiftMatch && altMatch && keyMatch
}

// ── 全局监听器 ────────────────────────────────────────────

let listenerAttached = false
const currentScope = ref<ShortcutScope>('global')

/**
 * 设置当前活跃的作用域
 */
export function setCurrentScope(scope: ShortcutScope): void {
  currentScope.value = scope
}

function handleKeyDown(e: KeyboardEvent): void {
  // 忽略系统快捷键
  if (e.metaKey && e.key === 'r') return // 刷新
  if (e.metaKey && e.key === 'w') return // 关闭标签

  const activeEl = document.activeElement
  const isInput = activeEl instanceof HTMLInputElement ||
    activeEl instanceof HTMLTextAreaElement ||
    activeEl?.getAttribute('contenteditable') === 'true'

  for (const shortcut of registry.values()) {
    if (shortcut.enabled === false) continue

    // 作用域匹配：global 始终生效，其他需要当前 scope 匹配
    const scopeMatch = shortcut.scope === 'global' || shortcut.scope === currentScope.value
    if (!scopeMatch) continue

    // 输入框内跳过，除非 allowInInput
    if (isInput && !shortcut.allowInInput) continue

    const parsed = parseKeyCombo(shortcut.keys)
    if (matchKeyEvent(e, parsed)) {
      e.preventDefault()
      e.stopPropagation()
      shortcut.action()
      return
    }
  }
}

/**
 * 在 Vue 组件中使用：自动管理全局监听器
 */
export function useShortcuts() {
  onMounted(() => {
    if (!listenerAttached) {
      document.addEventListener('keydown', handleKeyDown, true)
      listenerAttached = true
    }
  })

  onBeforeUnmount(() => {
    // 仅在没有其他组件使用时移除
    // 实际项目中可引用计数，此处简化处理
  })

  return {
    registerShortcut,
    unregisterShortcut,
    getShortcutsByScope,
    getAllShortcuts,
    setCurrentScope,
  }
}
