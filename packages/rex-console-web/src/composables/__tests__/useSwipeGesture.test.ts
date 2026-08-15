import { describe, it, expect, vi } from 'vitest'
import { defineComponent, ref } from 'vue'
import { mount } from '@vue/test-utils'
import { useSwipeGesture, type SwipeOptions } from '../useSwipeGesture'

function makeTouch(clientX: number, clientY: number): Touch {
  return { clientX, clientY } as unknown as Touch
}

function fireTouch(el: Element, type: 'touchstart' | 'touchend', touch: Touch) {
  const event = new Event(type, { bubbles: true })
  Object.defineProperty(event, 'touches', { value: type === 'touchstart' ? [touch] : [] })
  Object.defineProperty(event, 'changedTouches', { value: [touch] })
  el.dispatchEvent(event)
}

function makeTouchFn(touch: Touch) {
  return () => touch
}

function mountSwipe(options: SwipeOptions) {
  const elRef = ref<HTMLElement | null>(null)
  const wrapper = mount(defineComponent({
    setup() {
      const r = useSwipeGesture(elRef, options)
      return { elRef, ...r }
    },
    template: '<div ref="elRef" />',
  }))
  return { wrapper, el: wrapper.vm.elRef as HTMLElement }
}

describe('useSwipeGesture', () => {
  it('returns isSwiping ref starting false', () => {
    const { wrapper } = mountSwipe({})
    expect(wrapper.vm.isSwiping).toBe(false)
  })

  it('flags isSwiping on touchstart and clears on touchend', () => {
    const { wrapper, el } = mountSwipe({})
    fireTouch(el, 'touchstart', makeTouch(100, 100))
    expect(wrapper.vm.isSwiping).toBe(true)
    fireTouch(el, 'touchend', makeTouch(100, 100))
    expect(wrapper.vm.isSwiping).toBe(false)
  })

  it('ignores touchend when no touchstart happened', () => {
    const onSwipeLeft = vi.fn()
    const { wrapper, el } = mountSwipe({ onSwipeLeft })
    fireTouch(el, 'touchend', makeTouch(0, 0))
    expect(wrapper.vm.isSwiping).toBe(false)
    expect(onSwipeLeft).not.toHaveBeenCalled()
  })

  it('triggers onSwipeRight on a rightward horizontal swipe', () => {
    const onSwipeRight = vi.fn()
    const { el } = mountSwipe({ onSwipeRight, threshold: 50 })
    fireTouch(el, 'touchstart', makeTouch(100, 100))
    fireTouch(el, 'touchend', makeTouch(300, 110))
    expect(onSwipeRight).toHaveBeenCalledTimes(1)
  })

  it('triggers onSwipeLeft on a leftward horizontal swipe', () => {
    const onSwipeLeft = vi.fn()
    const { el } = mountSwipe({ onSwipeLeft, threshold: 50 })
    fireTouch(el, 'touchstart', makeTouch(300, 100))
    fireTouch(el, 'touchend', makeTouch(50, 110))
    expect(onSwipeLeft).toHaveBeenCalledTimes(1)
  })

  it('triggers onSwipeDown on a downward vertical swipe', () => {
    const onSwipeDown = vi.fn()
    const { el } = mountSwipe({ onSwipeDown, threshold: 50 })
    fireTouch(el, 'touchstart', makeTouch(100, 100))
    fireTouch(el, 'touchend', makeTouch(110, 300))
    expect(onSwipeDown).toHaveBeenCalledTimes(1)
  })

  it('triggers onSwipeUp on an upward vertical swipe', () => {
    const onSwipeUp = vi.fn()
    const { el } = mountSwipe({ onSwipeUp, threshold: 50 })
    fireTouch(el, 'touchstart', makeTouch(100, 300))
    fireTouch(el, 'touchend', makeTouch(110, 50))
    expect(onSwipeUp).toHaveBeenCalledTimes(1)
  })

  it('does not trigger when distance is below threshold', () => {
    const onSwipeRight = vi.fn()
    const { el } = mountSwipe({ onSwipeRight, threshold: 50 })
    fireTouch(el, 'touchstart', makeTouch(100, 100))
    fireTouch(el, 'touchend', makeTouch(120, 105))
    expect(onSwipeRight).not.toHaveBeenCalled()
  })

  it('does not trigger when the gesture is too slow (> 300ms)', () => {
    const onSwipeRight = vi.fn()
    const { el } = mountSwipe({ onSwipeRight, threshold: 50 })
    const start = makeTouch(100, 100)
    fireTouch(el, 'touchstart', start)
    // 模拟慢速：直接构造一个 changedTouch 但让 dt 超过 300ms
    // onTouchStart 记录 startTime=Date.now()，这里无法控制真实时间，
    // 改为断言“水平距离足够但被时间窗口过滤”的逻辑分支已被覆盖：
    // 用超大水平位移 + 让 Date.now 在两次调用间推进 > 300ms
    const realNow = Date.now
    let t = realNow()
    Date.now = () => { t += 400; return t }
    const end = makeTouch(500, 100)
    fireTouch(el, 'touchend', end)
    Date.now = realNow
    expect(onSwipeRight).not.toHaveBeenCalled()
  })

  it('prefers horizontal over vertical when diagonal', () => {
    const onSwipeRight = vi.fn()
    const onSwipeDown = vi.fn()
    const { el } = mountSwipe({ onSwipeRight, onSwipeDown, threshold: 50 })
    fireTouch(el, 'touchstart', makeTouch(100, 100))
    fireTouch(el, 'touchend', makeTouch(300, 160))
    expect(onSwipeRight).toHaveBeenCalledTimes(1)
    expect(onSwipeDown).not.toHaveBeenCalled()
  })
})
