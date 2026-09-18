import { afterEach, describe, expect, it, vi } from 'vitest'
import { clipboard } from './clipboard'

const originalClipboard = Object.getOwnPropertyDescriptor(navigator, 'clipboard')
const originalExecCommand = Object.getOwnPropertyDescriptor(document, 'execCommand')
afterEach(() => {
  if (originalClipboard) Object.defineProperty(navigator, 'clipboard', originalClipboard)
  else Reflect.deleteProperty(navigator, 'clipboard')
  if (originalExecCommand) Object.defineProperty(document, 'execCommand', originalExecCommand)
  else Reflect.deleteProperty(document, 'execCommand')
  document.body.replaceChildren()
})
function setupFallback(command: () => boolean) {
  Object.defineProperty(navigator, 'clipboard', { configurable: true, value: undefined })
  Object.defineProperty(document, 'execCommand', { configurable: true, value: command })
}
describe('clipboard', () => {
  it('copies without Clipboard API and restores input focus and selection', async () => {
    const command = vi.fn(() => true)
    setupFallback(command)
    const input = document.createElement('input')
    input.value = 'selected text'
    document.body.append(input)
    input.focus()
    input.setSelectionRange(1, 5)
    expect(await clipboard.writeText('copy me')).toBe(true)
    expect(command).toHaveBeenCalledWith('copy')
    expect(document.activeElement).toBe(input)
    expect(input.selectionStart).toBe(1)
    expect(input.selectionEnd).toBe(5)
    expect(document.querySelector('textarea')).toBeNull()
  })
  it('removes the temporary textarea when copy throws', async () => {
    setupFallback(() => { throw new Error('denied') })
    expect(await clipboard.writeText('text')).toBe(false)
    expect(document.querySelector('textarea')).toBeNull()
  })
  it('falls back after an asynchronous clipboard rejection', async () => {
    const command = vi.fn(() => true)
    setupFallback(command)
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: {
      writeText: vi.fn().mockRejectedValue(new Error('denied')),
    } })
    expect(await clipboard.writeText('text')).toBe(true)
    expect(command).toHaveBeenCalledOnce()
  })
  it('does not pretend to read the clipboard without browser permission', async () => {
    setupFallback(() => false)
    expect(await clipboard.readText()).toBeNull()
  })
})
