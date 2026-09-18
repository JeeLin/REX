// 剪贴板辅助：优先使用 navigator.clipboard（需安全上下文 / HTTPS），
// 在非安全上下文（HTTP、localhost 例外除外）降级到 textarea + execCommand。

async function writeText(text: string): Promise<boolean> {
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text)
      return true
    }
  } catch {
    // 安全上下文受限，降级
  }
  const active = document.activeElement instanceof HTMLElement ? document.activeElement : null
  const selection = window.getSelection()
  const ranges = selection ? Array.from({ length: selection.rangeCount }, (_, i) => selection.getRangeAt(i).cloneRange()) : []
  const input = active instanceof HTMLInputElement || active instanceof HTMLTextAreaElement ? active : null
  const start = input?.selectionStart ?? null
  const end = input?.selectionEnd ?? null
  const direction = input?.selectionDirection ?? undefined
  const ta = document.createElement('textarea')
  try {
    ta.value = text
    ta.style.position = 'fixed'
    ta.style.left = '-9999px'
    document.body.appendChild(ta)
    ta.focus()
    ta.select()
    return document.execCommand('copy')
  } catch {
    return false
  } finally {
    ta.remove()
    if (active?.isConnected) active.focus({ preventScroll: true })
    if (selection) {
      selection.removeAllRanges()
      for (const range of ranges) selection.addRange(range)
    }
    if (input?.isConnected && start !== null && end !== null) {
      input.setSelectionRange(start, end, direction)
    }
  }
}

async function readText(): Promise<string | null> {
  try {
    if (navigator.clipboard?.readText) {
      return await navigator.clipboard.readText()
    }
  } catch {
    // 安全上下文受限或用户拒绝
  }
  return null
}

export const clipboard = { writeText, readText }
