import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { defineComponent, ref } from 'vue'
import { mount } from '@vue/test-utils'
import { useVirtualKeyboard } from '../useVirtualKeyboard'

function installVisualViewport(initialHeight: number) {
  const listeners: Record<string, ((e?: unknown) => void)[]> = {}
  const vv = {
    height: initialHeight,
    addEventListener: (type: string, cb: (e?: unknown) => void) => {
      ;(listeners[type] ||= []).push(cb)
    },
    removeEventListener: (type: string, cb: (e?: unknown) => void) => {
      listeners[type] = (listeners[type] || []).filter((x) => x !== cb)
    },
  }
  Object.defineProperty(window, 'visualViewport', { configurable: true, value: vv })
  Object.defineProperty(window, 'innerHeight', { configurable: true, value: initialHeight })
  return {
    vv,
    fire() {
      ;(listeners['resize'] || []).forEach((cb) => cb())
    },
  }
}

describe('useVirtualKeyboard', () => {
  afterEach(() => {
    // 还原，避免影响其它测试
    Object.defineProperty(window, 'visualViewport', { configurable: true, value: undefined })
  })

  it('starts hidden when viewport equals window height', () => {
    const { vv } = installVisualViewport(800)
    const wrapper = mount(defineComponent({
      setup() {
        return { ...useVirtualKeyboard() }
      },
      template: '<div />',
    }))
    expect(wrapper.vm.isKeyboardVisible).toBe(false)
    expect(vv.height).toBe(800)
  })

  it('detects keyboard when visualViewport shrinks below 75%', () => {
    const ctx = installVisualViewport(800)
    const wrapper = mount(defineComponent({
      setup() {
        return { ...useVirtualKeyboard() }
      },
      template: '<div />',
    }))
    // 视口明显变小（键盘弹出）
    ctx.vv.height = 300
    ctx.fire()
    expect(wrapper.vm.isKeyboardVisible).toBe(true)
  })

  it('clears keyboard flag when viewport restores', () => {
    const ctx = installVisualViewport(800)
    const wrapper = mount(defineComponent({
      setup() {
        return { ...useVirtualKeyboard() }
      },
      template: '<div />',
    }))
    ctx.vv.height = 300
    ctx.fire()
    expect(wrapper.vm.isKeyboardVisible).toBe(true)
    ctx.vv.height = 800
    ctx.fire()
    expect(wrapper.vm.isKeyboardVisible).toBe(false)
  })

  it('removes its listener on unmount', () => {
    const ctx = installVisualViewport(800)
    const wrapper = mount(defineComponent({
      setup() {
        return { ...useVirtualKeyboard() }
      },
      template: '<div />',
    }))
    wrapper.unmount()
    ctx.vv.height = 300
    // 卸载后监听器已移除，不应再改变状态
    ctx.fire()
    expect(wrapper.vm.isKeyboardVisible).toBe(false)
  })

  it('falls back to window resize listener when visualViewport is unavailable', () => {
    Object.defineProperty(window, 'visualViewport', { configurable: true, value: undefined })
    Object.defineProperty(window, 'innerHeight', { configurable: true, value: 800 })
    const wrapper = mount(defineComponent({
      setup() {
        return { ...useVirtualKeyboard() }
      },
      template: '<div />',
    }))
    // 没有 visualViewport：监听挂在 window 上，handler 走 fallback 分支。
    // 注意：无独立 visualViewport 时 vh 等于 innerHeight，无法检测到键盘，
    // 但这里只验证 fallback 分支被挂载且不抛错。
    expect(() => window.dispatchEvent(new Event('resize'))).not.toThrow()
    expect(wrapper.vm.isKeyboardVisible).toBe(false)
  })
})
