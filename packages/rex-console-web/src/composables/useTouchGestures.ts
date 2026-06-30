interface TouchGestureOptions {
  onSwipeUp?: () => void
  onSwipeDown?: () => void
  onSwipeLeft?: () => void
  onSwipeRight?: () => void
  onLongPress?: (event: TouchEvent) => void
  onDoubleTap?: () => void
  swipeThreshold?: number
  longPressDelay?: number
  doubleTapDelay?: number
}

/**
 * Attach touch gesture handlers to an element.
 * Returns a cleanup function that removes all listeners.
 * Call cleanup in `onBeforeUnmount` or when the element is removed.
 */
export function useTouchGestures(element: HTMLElement, options: TouchGestureOptions) {
  const {
    swipeThreshold = 50,
    longPressDelay = 500,
    doubleTapDelay = 300,
    onSwipeUp,
    onSwipeDown,
    onSwipeLeft,
    onSwipeRight,
    onLongPress,
    onDoubleTap
  } = options

  let touchStart: { x: number; y: number; time: number } | null = null
  let longPressTimer: number | null = null
  let lastTapTime = 0

  function handleTouchStart(event: TouchEvent) {
    const touch = event.touches[0]
    touchStart = {
      x: touch.clientX,
      y: touch.clientY,
      time: Date.now()
    }

    // Start long press timer
    if (onLongPress) {
      longPressTimer = window.setTimeout(() => {
        onLongPress(event)
      }, longPressDelay)
    }
  }

  function handleTouchMove(event: TouchEvent) {
    if (!touchStart) return

    const touch = event.touches[0]
    const deltaX = touch.clientX - touchStart.x
    const deltaY = touch.clientY - touchStart.y
    const distance = Math.sqrt(deltaX * deltaX + deltaY * deltaY)

    // Cancel long press if moved too far
    if (distance > 10 && longPressTimer) {
      clearTimeout(longPressTimer)
      longPressTimer = null
    }
  }

  function handleTouchEnd(event: TouchEvent) {
    if (!touchStart) return

    // Clear long press timer
    if (longPressTimer) {
      clearTimeout(longPressTimer)
      longPressTimer = null
    }

    const touch = event.changedTouches[0]
    const deltaX = touch.clientX - touchStart.x
    const deltaY = touch.clientY - touchStart.y
    const deltaTime = Date.now() - touchStart.time

    // Check for double tap
    if (onDoubleTap && deltaTime < 300 && Math.abs(deltaX) < 10 && Math.abs(deltaY) < 10) {
      const now = Date.now()
      if (now - lastTapTime < doubleTapDelay) {
        onDoubleTap()
        lastTapTime = 0
        touchStart = null
        return
      }
      lastTapTime = now
    }

    // Check for swipe gestures (only if movement was significant and fast enough)
    if (deltaTime < 500) {
      const distance = Math.sqrt(deltaX * deltaX + deltaY * deltaY)

      if (distance > swipeThreshold) {
        const angle = Math.atan2(deltaY, deltaX)

        // Determine swipe direction based on angle
        if (angle > -Math.PI / 4 && angle <= Math.PI / 4) {
          onSwipeRight?.()
        } else if (angle > Math.PI / 4 && angle <= (3 * Math.PI) / 4) {
          onSwipeDown?.()
        } else if (angle > -(3 * Math.PI) / 4 && angle <= -Math.PI / 4) {
          onSwipeUp?.()
        } else {
          onSwipeLeft?.()
        }
      }
    }

    touchStart = null
  }

  // Attach immediately (element must already exist)
  element.addEventListener('touchstart', handleTouchStart, { passive: true })
  element.addEventListener('touchmove', handleTouchMove, { passive: true })
  element.addEventListener('touchend', handleTouchEnd, { passive: true })

  // Return cleanup function
  return function cleanup() {
    element.removeEventListener('touchstart', handleTouchStart)
    element.removeEventListener('touchmove', handleTouchMove)
    element.removeEventListener('touchend', handleTouchEnd)
    if (longPressTimer) {
      clearTimeout(longPressTimer)
    }
  }
}
