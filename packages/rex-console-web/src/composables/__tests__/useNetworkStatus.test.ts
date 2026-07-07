import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { defineComponent, nextTick } from 'vue'
import { mount } from '@vue/test-utils'
import { useNetworkStatus } from '../useNetworkStatus'

describe('useNetworkStatus', () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  function mountWithNetwork() {
    const Comp = defineComponent({
      setup() {
        return useNetworkStatus()
      },
      template: '<div data-testid="status">{{ isOnline }}</div>',
    })
    return mount(Comp)
  }

  it('exposes isOnline ref', () => {
    const wrapper = mountWithNetwork()
    const text = wrapper.find('[data-testid="status"]').text()
    // navigator.onLine is true by default in jsdom
    expect(text).toBe('true')
  })

  it('responds to offline event', async () => {
    const wrapper = mountWithNetwork()
    window.dispatchEvent(new Event('offline'))
    await nextTick()
    expect(wrapper.find('[data-testid="status"]').text()).toBe('false')
  })

  it('responds to online event after going offline', async () => {
    const wrapper = mountWithNetwork()
    window.dispatchEvent(new Event('offline'))
    await nextTick()
    expect(wrapper.find('[data-testid="status"]').text()).toBe('false')

    window.dispatchEvent(new Event('online'))
    await nextTick()
    expect(wrapper.find('[data-testid="status"]').text()).toBe('true')
  })

  it('toggles between states', async () => {
    const wrapper = mountWithNetwork()
    expect(wrapper.find('[data-testid="status"]').text()).toBe('true')

    window.dispatchEvent(new Event('offline'))
    await nextTick()
    expect(wrapper.find('[data-testid="status"]').text()).toBe('false')

    window.dispatchEvent(new Event('online'))
    await nextTick()
    expect(wrapper.find('[data-testid="status"]').text()).toBe('true')

    window.dispatchEvent(new Event('offline'))
    await nextTick()
    expect(wrapper.find('[data-testid="status"]').text()).toBe('false')
  })
})
