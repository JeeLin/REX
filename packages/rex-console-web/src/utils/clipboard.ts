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
  try {
    const ta = document.createElement('textarea')
    ta.value = text
    ta.style.position = 'fixed'
    ta.style.left = '-9999px'
    document.body.appendChild(ta)
    ta.focus()
    ta.select()
    const ok = document.execCommand('copy')
    document.body.removeChild(ta)
    return ok
  } catch {
    return false
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
