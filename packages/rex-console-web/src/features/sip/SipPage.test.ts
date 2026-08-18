import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { nextTick } from 'vue'
import SipPage from './SipPage.vue'
import { i18n } from '@/i18n'

// 用 stub 的 SipClient 控制事件注入，验证质量事件驱动卡片渲染（子任务 #5）。
vi.mock('@/api/sip', async () => {
  const actual = await vi.importActual<typeof import('@/api/sip')>('@/api/sip')
  class StubSipClient {
    handlers: any
    constructor(_rid: string, handlers: any) {
      this.handlers = handlers
    }
    connect() {}
    close() {}
  }
  return { ...actual, SipClient: StubSipClient }
})

describe('SipPage quality card', () => {
  beforeEach(() => localStorage.setItem('rex-token', 'tok'))
  afterEach(() => vi.restoreAllMocks())

  it('shows quality metrics when active call receives sip.quality', async () => {
    const wrapper = mount(SipPage, {
      props: { resourceId: 'r1', name: 'SIP' },
      global: { stubs: ['Dialpad', 'CallState'], plugins: [i18n] },
    })
    await flushPromises()

    const client = (wrapper.vm as any).client as any
    // 先进入 active 通话
    client.handlers.onEvent({
      type: 'sip.call_state',
      payload: { callId: 'c1', state: 'active' },
    })
    await nextTick()
    // 再注入质量指标
    client.handlers.onEvent({
      type: 'sip.quality',
      payload: { loss: 0.1, jitter: 5.5, rtt: 42.0 },
    })
    await nextTick()

    expect(wrapper.text()).toContain('10.0%') // loss 0.1 → 10.0%
    expect(wrapper.text()).toContain('5.5 ms')
    expect(wrapper.text()).toContain('42.0 ms')
  })

  it('hides quality card before any quality event', async () => {
    const wrapper = mount(SipPage, {
      props: { resourceId: 'r1', name: 'SIP' },
      global: { stubs: ['Dialpad', 'CallState'], plugins: [i18n] },
    })
    await flushPromises()
    client(wrapper).handlers.onEvent({
      type: 'sip.call_state',
      payload: { callId: 'c1', state: 'active' },
    })
    await nextTick()
    expect(wrapper.find('.quality-card').exists()).toBe(false)
  })
})

function client(wrapper: any) {
  return (wrapper.vm as any).client as any
}
