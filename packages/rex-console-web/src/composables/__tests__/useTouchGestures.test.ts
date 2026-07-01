import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { useTouchGestures } from '../useTouchGestures'

// Create a minimal mock element that supports addEventListener/removeEventListener
type Listener = (...args: unknown[]) => void

function createMockElement() {
  const listeners: Record<string, Listener[]> = {}
  return {
    el: {
      addEventListener: vi.fn((event: string, handler: Listener, _opts?: AddEventListenerOptions) => {
        if (!listeners[event]) listeners[event] = []
        listeners[event].push(handler)
      }),
      removeEventListener: vi.fn((event: string, handler: Listener) => {
        if (listeners[event]) {
          listeners[event] = listeners[event].filter((h) => h !== handler)
        }
      }),
    },
    listeners,
    emit(event: string, touchEvent: Partial<TouchEvent>) {
      for (const handler of listeners[event] ?? []) {
        handler(touchEvent)
      }
    },
  }
}

function makeTouchEvent(
  touches: Array<{ clientX: number; clientY: number }>,
  changedTouches?: Array<{ clientX: number; clientY: number }>,
): Partial<TouchEvent> {
  const makeTouchList = (arr: Array<{ clientX: number; clientY: number }>) => {
    const list = arr.map((t, i) => ({
      clientX: t.clientX,
      clientY: t.clientY,
      identifier: i,
      target: null,
    })) as unknown as TouchList
    return list
  }
  return {
    touches: makeTouchList(touches),
    changedTouches: makeTouchList(changedTouches ?? touches),
  }
}

describe('useTouchGestures', () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
    vi.restoreAllMocks()
  })

  it('returns a cleanup function that removes listeners', () => {
    const { el, listeners } = createMockElement()
    const cleanup = useTouchGestures(el as unknown as HTMLElement, {})

    expect(el.addEventListener).toHaveBeenCalledTimes(3)
    expect(listeners['touchstart']?.length).toBe(1)
    expect(listeners['touchmove']?.length).toBe(1)
    expect(listeners['touchend']?.length).toBe(1)

    cleanup()

    expect(el.removeEventListener).toHaveBeenCalledTimes(3)
    expect(listeners['touchstart']?.length).toBe(0)
  })

  describe('swipe gestures', () => {
    it('fires onSwipeRight when swiping right quickly', () => {
      const { el, emit } = createMockElement()
      const onSwipeRight = vi.fn()
      useTouchGestures(el as unknown as HTMLElement, { onSwipeRight })

      emit('touchstart', makeTouchEvent([{ clientX: 100, clientY: 100 }]))
      emit('touchend', makeTouchEvent(
        [{ clientX: 100, clientY: 100 }],
        [{ clientX: 200, clientY: 105 }],
      ))

      expect(onSwipeRight).toHaveBeenCalledTimes(1)
    })

    it('fires onSwipeLeft when swiping left quickly', () => {
      const { el, emit } = createMockElement()
      const onSwipeLeft = vi.fn()
      useTouchGestures(el as unknown as HTMLElement, { onSwipeLeft })

      emit('touchstart', makeTouchEvent([{ clientX: 200, clientY: 100 }]))
      emit('touchend', makeTouchEvent(
        [{ clientX: 200, clientY: 100 }],
        [{ clientX: 100, clientY: 105 }],
      ))

      expect(onSwipeLeft).toHaveBeenCalledTimes(1)
    })

    it('does not fire swipe when movement is below threshold', () => {
      const { el, emit } = createMockElement()
      const onSwipeRight = vi.fn()
      useTouchGestures(el as unknown as HTMLElement, { onSwipeRight, swipeThreshold: 50 })

      emit('touchstart', makeTouchEvent([{ clientX: 100, clientY: 100 }]))
      emit('touchend', makeTouchEvent(
        [{ clientX: 100, clientY: 100 }],
        [{ clientX: 120, clientY: 105 }],
      ))

      expect(onSwipeRight).not.toHaveBeenCalled()
    })
  })

  describe('long press', () => {
    it('fires onLongPress after delay', () => {
      const { el, emit } = createMockElement()
      const onLongPress = vi.fn()
      useTouchGestures(el as unknown as HTMLElement, { onLongPress, longPressDelay: 500 })

      emit('touchstart', makeTouchEvent([{ clientX: 100, clientY: 100 }]))
      vi.advanceTimersByTime(500)

      expect(onLongPress).toHaveBeenCalledTimes(1)
    })

    it('cancels long press if finger moves', () => {
      const { el, emit } = createMockElement()
      const onLongPress = vi.fn()
      useTouchGestures(el as unknown as HTMLElement, { onLongPress, longPressDelay: 500 })

      emit('touchstart', makeTouchEvent([{ clientX: 100, clientY: 100 }]))
      emit('touchmove', makeTouchEvent(
        [{ clientX: 120, clientY: 120 }],
        [{ clientX: 120, clientY: 120 }],
      ))
      vi.advanceTimersByTime(500)

      expect(onLongPress).not.toHaveBeenCalled()
    })
  })

  describe('double tap', () => {
    it('fires onDoubleTap on two quick taps', () => {
      const { el, emit } = createMockElement()
      const onDoubleTap = vi.fn()
      useTouchGestures(el as unknown as HTMLElement, { onDoubleTap, doubleTapDelay: 300 })

      // First tap
      emit('touchstart', makeTouchEvent([{ clientX: 100, clientY: 100 }]))
      emit('touchend', makeTouchEvent(
        [{ clientX: 100, clientY: 100 }],
        [{ clientX: 100, clientY: 100 }],
      ))
      vi.advanceTimersByTime(100)

      // Second tap
      emit('touchstart', makeTouchEvent([{ clientX: 100, clientY: 100 }]))
      emit('touchend', makeTouchEvent(
        [{ clientX: 100, clientY: 100 }],
        [{ clientX: 100, clientY: 100 }],
      ))

      expect(onDoubleTap).toHaveBeenCalledTimes(1)
    })
  })

  describe('single tap', () => {
    it('fires onTap on a quick tap', () => {
      const { el, emit } = createMockElement()
      const onTap = vi.fn()
      useTouchGestures(el as unknown as HTMLElement, { onTap })

      emit('touchstart', makeTouchEvent([{ clientX: 100, clientY: 100 }]))
      emit('touchend', makeTouchEvent(
        [{ clientX: 100, clientY: 100 }],
        [{ clientX: 100, clientY: 100 }],
      ))

      expect(onTap).toHaveBeenCalledTimes(1)
    })

    it('does not fire onTap on a swipe', () => {
      const { el, emit } = createMockElement()
      const onTap = vi.fn()
      useTouchGestures(el as unknown as HTMLElement, { onTap })

      emit('touchstart', makeTouchEvent([{ clientX: 100, clientY: 100 }]))
      emit('touchend', makeTouchEvent(
        [{ clientX: 100, clientY: 100 }],
        [{ clientX: 200, clientY: 100 }],
      ))

      expect(onTap).not.toHaveBeenCalled()
    })
  })

  describe('pan gesture', () => {
    it('fires onPan on movement and onPanEnd on release', () => {
      const { el, emit } = createMockElement()
      const onPan = vi.fn()
      const onPanEnd = vi.fn()
      useTouchGestures(el as unknown as HTMLElement, { onPan, onPanEnd, tapThreshold: 10 })

      emit('touchstart', makeTouchEvent([{ clientX: 100, clientY: 100 }]))
      emit('touchmove', makeTouchEvent(
        [{ clientX: 130, clientY: 110 }],
      ))

      expect(onPan).toHaveBeenCalled()

      emit('touchend', makeTouchEvent(
        [{ clientX: 130, clientY: 110 }],
        [{ clientX: 130, clientY: 110 }],
      ))

      expect(onPanEnd).toHaveBeenCalledTimes(1)
    })
  })

  describe('pinch gesture', () => {
    it('fires onPinch with scale when two fingers move', () => {
      const { el, emit } = createMockElement()
      const onPinch = vi.fn()
      useTouchGestures(el as unknown as HTMLElement, { onPinch })

      // Two fingers start
      emit('touchstart', makeTouchEvent([
        { clientX: 100, clientY: 100 },
        { clientX: 200, clientY: 100 },
      ]))

      // Two fingers move apart
      emit('touchmove', makeTouchEvent([
        { clientX: 80, clientY: 100 },
        { clientX: 220, clientY: 100 },
      ]))

      expect(onPinch).toHaveBeenCalled()
      const [scale] = onPinch.mock.calls[0]
      expect(scale).toBeGreaterThan(1)
    })

    it('fires onPinchEnd when second finger lifts', () => {
      const { el, emit } = createMockElement()
      const onPinchEnd = vi.fn()
      const onPinch = vi.fn()
      useTouchGestures(el as unknown as HTMLElement, { onPinch, onPinchEnd })

      // Two fingers start
      emit('touchstart', makeTouchEvent([
        { clientX: 100, clientY: 100 },
        { clientX: 200, clientY: 100 },
      ]))

      // One finger lifts (touchend with 1 remaining touch)
      emit('touchend', makeTouchEvent(
        [{ clientX: 100, clientY: 100 }],
        [{ clientX: 200, clientY: 100 }],
      ))

      expect(onPinchEnd).toHaveBeenCalledTimes(1)
    })
  })
})
