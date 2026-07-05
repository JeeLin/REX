import { describe, it, expect, vi, beforeEach } from 'vitest'
import { useContextMenu, type MenuItem } from '../useContextMenu'

describe('useContextMenu', () => {
  beforeEach(() => {
    const { hide } = useContextMenu()
    hide()
  })

  it('initializes with hidden state', () => {
    const { visible, x, y, items } = useContextMenu()
    expect(visible.value).toBe(false)
    expect(x.value).toBe(0)
    expect(y.value).toBe(0)
    expect(items.value).toEqual([])
  })

  it('shows menu at click position', () => {
    const { visible, x, y, items, show } = useContextMenu()
    const menuItems: MenuItem[] = [
      { label: 'Copy', action: vi.fn() },
      { label: 'Paste', action: vi.fn() },
    ]
    const event = {
      clientX: 100,
      clientY: 200,
      preventDefault: vi.fn(),
      stopPropagation: vi.fn(),
    } as unknown as MouseEvent

    show(event, menuItems)

    expect(visible.value).toBe(true)
    expect(x.value).toBeGreaterThan(0)
    expect(y.value).toBeGreaterThan(0)
    expect(items.value).toEqual(menuItems)
    expect(event.preventDefault).toHaveBeenCalled()
    expect(event.stopPropagation).toHaveBeenCalled()
  })

  it('hides menu', () => {
    const { visible, show, hide } = useContextMenu()
    const event = {
      clientX: 50,
      clientY: 50,
      preventDefault: vi.fn(),
      stopPropagation: vi.fn(),
    } as unknown as MouseEvent

    show(event, [{ label: 'Test' }])
    expect(visible.value).toBe(true)

    hide()
    expect(visible.value).toBe(false)
  })

  it('adjusts position to stay within viewport', () => {
    const { x, y, show } = useContextMenu()
    const event = {
      clientX: window.innerWidth - 10,
      clientY: window.innerHeight - 10,
      preventDefault: vi.fn(),
      stopPropagation: vi.fn(),
    } as unknown as MouseEvent

    show(event, [{ label: 'Item' }, { label: 'Item2' }])

    expect(x.value).toBeLessThan(window.innerWidth)
    expect(y.value).toBeLessThan(window.innerHeight)
  })
})
