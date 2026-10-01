import { describe, it, expect } from 'vitest'
import { isTypingTarget } from '../isTypingTarget'

// The guard only accepts EventTarget | null, but callers can hand over
// anything the DOM event pipeline produced (or a test double).
function asTarget(value: unknown): EventTarget | null {
  return value as EventTarget | null
}

describe('isTypingTarget', () => {
  it('matches form controls and contenteditable elements', () => {
    const editable = document.createElement('div')
    editable.contentEditable = 'true'

    expect(isTypingTarget(document.createElement('input'))).toBe(true)
    expect(isTypingTarget(document.createElement('textarea'))).toBe(true)
    expect(isTypingTarget(document.createElement('select'))).toBe(true)
    expect(isTypingTarget(editable)).toBe(true)
  })

  it('rejects elements that never hold user input', () => {
    expect(isTypingTarget(document.createElement('div'))).toBe(false)
    expect(isTypingTarget(document.createElement('button'))).toBe(false)
    expect(isTypingTarget(document.createTextNode('x'))).toBe(false)
    // SVG elements are EventTargets but fail the HTMLElement guard.
    expect(isTypingTarget(document.createElementNS('http://www.w3.org/2000/svg', 'svg'))).toBe(false)
    expect(isTypingTarget(window)).toBe(false)
  })

  it('rejects non-element targets', () => {
    expect(isTypingTarget(null)).toBe(false)
    expect(isTypingTarget(asTarget(undefined))).toBe(false)
    expect(isTypingTarget(asTarget({ tagName: undefined }))).toBe(false)
    expect(isTypingTarget(asTarget({ tagName: 'INPUT' }))).toBe(false)
    expect(isTypingTarget(asTarget('INPUT'))).toBe(false)
  })
})
