import { describe, it, expect } from 'vitest'
import { terminalThemes, getTerminalTheme } from '../terminal-themes'

describe('terminal-themes', () => {
  it('exposes multiple built-in themes', () => {
    const names = terminalThemes.map((t) => t.name)
    expect(names).toContain('default')
    expect(names).toContain('ubuntu')
    expect(names).toContain('solarized-dark')
    expect(terminalThemes.length).toBeGreaterThanOrEqual(3)
  })

  it('every theme has a valid xterm ITheme (background + foreground)', () => {
    for (const t of terminalThemes) {
      expect(t.theme.background).toMatch(/^#[0-9A-Fa-f]{6}$/)
      expect(t.theme.foreground).toMatch(/^#[0-9A-Fa-f]{6}$/)
    }
  })

  it('getTerminalTheme returns the named theme', () => {
    expect(getTerminalTheme('ubuntu')).toBe(terminalThemes[1]!.theme)
  })

  it('getTerminalTheme falls back to default for unknown name', () => {
    expect(getTerminalTheme('does-not-exist')).toBe(terminalThemes[0]!.theme)
  })
})
