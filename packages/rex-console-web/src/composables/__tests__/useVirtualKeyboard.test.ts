import { describe, it, expect } from 'vitest'

describe('useVirtualKeyboard', () => {
  it('exports a composable function', async () => {
    const { useVirtualKeyboard } = await import('../useVirtualKeyboard')
    expect(typeof useVirtualKeyboard).toBe('function')
  })
})
