import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { ref, computed } from 'vue'
import RedisMonitor from '../RedisMonitor.vue'

// Mock vue-i18n
vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}))

// Mock useRedisSession
const mockConnected = ref(true)
const mockExecute = vi.fn()

vi.mock('../useRedisSession', () => ({
  useRedisSession: () => ({
    connected: mockConnected,
    execute: mockExecute,
  }),
}))

describe('RedisMonitor', () => {
  beforeEach(() => {
    mockConnected.value = true
    mockExecute.mockReset()
  })

  const createWrapper = (props = {}) => {
    return mount(RedisMonitor, {
      props: {
        resourceId: 'test-resource',
        ...props,
      },
      global: {
        stubs: {
          Teleport: true,
        },
      },
    })
  }

  it('renders monitor title', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('redis.monitor.title')
  })

  it('renders refresh button', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('common.refresh')
  })

  it('renders refresh interval selector', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('.monitor-interval-select').exists()).toBe(true)
  })

  it('shows not connected state when disconnected', () => {
    mockConnected.value = false
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('redis.monitor.notConnected')
  })

  it('shows error state when fetchInfo fails', async () => {
    mockExecute.mockRejectedValueOnce(new Error('Connection failed'))
    const wrapper = createWrapper()
    
    // Trigger fetchInfo by clicking refresh
    await wrapper.find('.monitor-refresh-btn').trigger('click')
    await wrapper.vm.$nextTick()
    
    expect(wrapper.text()).toContain('Connection failed')
  })
})