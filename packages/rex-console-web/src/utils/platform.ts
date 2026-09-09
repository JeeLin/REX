/**
 * 平台检测工具
 * macOS 用 Cmd (Meta)，Windows/Linux 用 Ctrl
 */

/**
 * 检测当前是否为 macOS 平台
 */
export function isMac(): boolean {
  if (typeof navigator === 'undefined') return false
  return navigator.platform.toUpperCase().includes('MAC') ||
    navigator.userAgent.toUpperCase().includes('MAC')
}

/**
 * 获取修饰键名称
 * macOS: 'Meta' (Cmd), 其他: 'Control' (Ctrl)
 */
export function modKey(): 'Meta' | 'Control' {
  return isMac() ? 'Meta' : 'Control'
}

/**
 * 获取修饰键的显示标签
 * macOS: '⌘', 其他: 'Ctrl'
 */
export function modLabel(): string {
  return isMac() ? '⌘' : 'Ctrl'
}

/**
 * 格式化快捷键为用户友好的显示文本
 * @example formatShortcut('Mod+K') // macOS: '⌘K', 其他: 'Ctrl+K'
 */
export function formatShortcut(keys: string): string {
  return keys
    .replace(/Mod/g, modLabel())
    .replace(/Alt/g, isMac() ? '⌥' : 'Alt')
    .replace(/Shift/g, isMac() ? '⇧' : 'Shift')
    .replace(/\+/g, '')
}
