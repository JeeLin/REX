import { onMounted, onUnmounted } from 'vue'

interface Shortcut {
  key: string
  ctrl?: boolean
  shift?: boolean
  alt?: boolean
  handler: () => void
}

export function useKeyboardShortcuts(shortcuts: Shortcut[]) {
  const onKeydown = (e: KeyboardEvent) => {
    for (const s of shortcuts) {
      const ctrlMatch = s.ctrl ? (e.ctrlKey || e.metaKey) : !e.ctrlKey && !e.metaKey
      const shiftMatch = s.shift ? e.shiftKey : !e.shiftKey
      const altMatch = s.alt ? e.altKey : !e.altKey
      if (
        e.key.toLowerCase() === s.key.toLowerCase() &&
        ctrlMatch &&
        shiftMatch &&
        altMatch
      ) {
        e.preventDefault()
        s.handler()
      }
    }
  }

  onMounted(() => window.addEventListener('keydown', onKeydown))
  onUnmounted(() => window.removeEventListener('keydown', onKeydown))
}
