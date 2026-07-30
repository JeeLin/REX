import { describe, it, expect, vi } from 'vitest'
import { ref } from 'vue'

// useSwipeGesture requires DOM touch events, test with mock
describe('useSwipeGesture', () => {
  it('exports a composable function', async () => {
    const { useSwipeGesture } = await import('../useSwipeGesture')
    expect(typeof useSwipeGesture).toBe('function')
  })

  it('returns isSwiping ref', async () => {
    const { useSwipeGesture } = await import('../useSwipeGesture')
    const el = ref(document.createElement('div'))
    const { isSwiping } = useSwipeGesture(el, {})
    expect(isSwiping.value).toBe(false)
  })
})
