export interface TouchGestureOptions {
  // Swipe gestures
  onSwipeUp?: () => void
  onSwipeDown?: () => void
  onSwipeLeft?: () => void
  onSwipeRight?: () => void
  // Tap gestures
  onTap?: (event: TouchEvent) => void
  onDoubleTap?: () => void
  onLongPress?: (event: TouchEvent) => void
  // Pinch gesture (two-finger zoom)
  onPinch?: (scale: number, deltaScale: number) => void
  onPinchEnd?: (finalScale: number) => void
  // Pan gesture (single-finger continuous drag)
  onPan?: (deltaX: number, deltaY: number) => void
  onPanEnd?: (deltaX: number, deltaY: number) => void
  // Thresholds
  swipeThreshold?: number
  longPressDelay?: number
  doubleTapDelay?: number
  tapThreshold?: number
}

interface TouchPoint {
  x: number
  y: number
  time: number
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
    tapThreshold = 10,
    onSwipeUp,
    onSwipeDown,
    onSwipeLeft,
    onSwipeRight,
    onTap,
    onDoubleTap,
    onLongPress,
    onPinch,
    onPinchEnd,
    onPan,
    onPanEnd,
  } = options

  let touchStart: TouchPoint | null = null
  let longPressTimer: number | null = null
  let lastTapTime = 0
  let gestureActive = false // true once a recognized gesture starts
  let pinchStartDistance: number | null = null
  let pinchStartScale = 1

  // -- helpers --

  function getDistance(a: Touch, b: Touch) {
    const dx = a.clientX - b.clientX
    const dy = a.clientY - b.clientY
    return Math.sqrt(dx * dx + dy * dy)
  }

  function clearLongPress() {
    if (longPressTimer !== null) {
      clearTimeout(longPressTimer)
      longPressTimer = null
    }
  }

  // -- event handlers --

  function handleTouchStart(event: TouchEvent) {
    if (event.touches.length === 2 && onPinch) {
      // Pinch starts: ignore previous single-finger state
      clearLongPress()
      gestureActive = true
      pinchStartDistance = getDistance(event.touches[0], event.touches[1])
      pinchStartScale = 1
      touchStart = null
      return
    }

    const touch = event.touches[0]
    touchStart = {
      x: touch.clientX,
      y: touch.clientY,
      time: Date.now(),
    }
    gestureActive = false

    // Start long press timer (single finger only)
    if (onLongPress) {
      longPressTimer = window.setTimeout(() => {
        clearLongPress()
        onLongPress(event)
      }, longPressDelay)
    }
  }

  function handleTouchMove(event: TouchEvent) {
    // --- Pinch handling (two fingers) ---
    if (event.touches.length === 2 && pinchStartDistance !== null && onPinch) {
      clearLongPress()
      const currentDistance = getDistance(event.touches[0], event.touches[1])
      const scale = currentDistance / pinchStartDistance
      const deltaScale = scale - pinchStartScale
      pinchStartScale = scale
      onPinch(scale, deltaScale)
      return
    }

    if (!touchStart) return

    const touch = event.touches[0]
    const deltaX = touch.clientX - touchStart.x
    const deltaY = touch.clientY - touchStart.y
    const distance = Math.sqrt(deltaX * deltaX + deltaY * deltaY)

    // Cancel long press if moved too far
    if (distance > tapThreshold && longPressTimer) {
      clearLongPress()
    }

    // Pan gesture: fire on every move once past threshold
    if (onPan && distance > tapThreshold) {
      gestureActive = true
      onPan(deltaX, deltaY)
    }
  }

  function handleTouchEnd(event: TouchEvent) {
    // --- Pinch end ---
    if (pinchStartDistance !== null) {
      if (event.touches.length < 2) {
        pinchStartDistance = null
        onPinchEnd?.(pinchStartScale)
        pinchStartScale = 1
        touchStart = null
        gestureActive = false
        return
      }
      return // still two fingers down
    }

    clearLongPress()

    if (!touchStart) return

    const touch = event.changedTouches[0]
    const deltaX = touch.clientX - touchStart.x
    const deltaY = touch.clientY - touchStart.y
    const distance = Math.sqrt(deltaX * deltaX + deltaY * deltaY)
    const deltaTime = Date.now() - touchStart.time

    // --- Pan end ---
    if (onPan && gestureActive) {
      onPanEnd?.(deltaX, deltaY)
      touchStart = null
      gestureActive = false
      return
    }

    // --- Double tap ---
    if (onDoubleTap && deltaTime < 300 && distance < tapThreshold) {
      const now = Date.now()
      if (now - lastTapTime < doubleTapDelay) {
        onDoubleTap()
        lastTapTime = 0
        touchStart = null
        return
      }
      lastTapTime = now
    }

    // --- Single tap ---
    if (onTap && deltaTime < 300 && distance < tapThreshold) {
      onTap(event)
      touchStart = null
      return
    }

    // --- Swipe (only if movement was significant and fast enough) ---
    if (deltaTime < 500 && distance > swipeThreshold) {
      const angle = Math.atan2(deltaY, deltaX)

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

    touchStart = null
    gestureActive = false
  }

  // Attach listeners (element must already exist)
  element.addEventListener('touchstart', handleTouchStart, { passive: true })
  element.addEventListener('touchmove', handleTouchMove, { passive: true })
  element.addEventListener('touchend', handleTouchEnd, { passive: true })

  // Return cleanup function
  return function cleanup() {
    element.removeEventListener('touchstart', handleTouchStart)
    element.removeEventListener('touchmove', handleTouchMove)
    element.removeEventListener('touchend', handleTouchEnd)
    clearLongPress()
    pinchStartDistance = null
    touchStart = null
  }
}
