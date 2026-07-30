import { ref, onMounted, onBeforeUnmount, type Ref } from 'vue'

export interface SwipeOptions {
  threshold?: number
  onSwipeLeft?: () => void
  onSwipeRight?: () => void
  onSwipeDown?: () => void
  onSwipeUp?: () => void
}

export function useSwipeGesture(
  elementRef: Ref<HTMLElement | null>,
  options: SwipeOptions = {},
) {
  const { threshold = 50, onSwipeLeft, onSwipeRight, onSwipeDown, onSwipeUp } = options

  const isSwiping = ref(false)
  let startX = 0
  let startY = 0
  let startTime = 0

  function onTouchStart(e: TouchEvent) {
    const touch = e.touches[0]
    startX = touch.clientX
    startY = touch.clientY
    startTime = Date.now()
    isSwiping.value = true
  }

  function onTouchEnd(e: TouchEvent) {
    if (!isSwiping.value) return
    isSwiping.value = false

    const touch = e.changedTouches[0]
    const dx = touch.clientX - startX
    const dy = touch.clientY - startY
    const dt = Date.now() - startTime

    // 快速滑动（< 300ms）且距离足够
    if (dt > 300) return

    const absDx = Math.abs(dx)
    const absDy = Math.abs(dy)

    if (absDx > absDy && absDx > threshold) {
      // 水平滑动
      if (dx > 0) {
        onSwipeRight?.()
      } else {
        onSwipeLeft?.()
      }
    } else if (absDy > absDx && absDy > threshold) {
      // 垂直滑动
      if (dy > 0) {
        onSwipeDown?.()
      } else {
        onSwipeUp?.()
      }
    }
  }

  onMounted(() => {
    const el = elementRef.value
    if (!el) return
    el.addEventListener('touchstart', onTouchStart, { passive: true })
    el.addEventListener('touchend', onTouchEnd, { passive: true })
  })

  onBeforeUnmount(() => {
    const el = elementRef.value
    if (!el) return
    el.removeEventListener('touchstart', onTouchStart)
    el.removeEventListener('touchend', onTouchEnd)
  })

  return { isSwiping }
}
