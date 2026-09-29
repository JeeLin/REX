// Shared guard: skip global shortcuts while the user is typing in a
// form control or contenteditable region. Consolidates the three
// previously duplicated inline checks (useKeyboardShortcuts, FilesPage,
// WorkspacePage split handler).
export function isTypingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false
  return target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable
}
