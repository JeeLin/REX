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

// 多账户切换（0.70.4）：挂载资源含多个账户时显示下拉，切换触发 update 写回 activeAccount。
// 用 vi.hoisted 提升常量，避免 vi.mock 工厂被 hoist 到其定义之前而引用未初始化变量。
const { sipProfile } = vi.hoisted(() => ({
  sipProfile: JSON.stringify({
    accounts: [
      { id: 'a1', server: 'pbx.example.com', username: 'alice', displayName: 'Alice' },
      { id: 'a2', server: 'pbx2.example.com', username: 'bob', displayName: 'Bob' },
    ],
    activeAccount: 'a1',
  }),
}))

vi.mock('@/api/resources', () => ({
  resourcesApi: {
    get: vi.fn().mockResolvedValue({
      id: 'r1',
      environment_id: 'env1',
      name: 'Phone',
      protocol: 'sip',
      host: 'pbx.example.com',
      port: 5060,
      username: 'alice',
      config_json: sipProfile,
      color: null,
      sort_order: 0,
      created_at: '',
      updated_at: '',
    }),
    update: vi.fn().mockResolvedValue({}),
  },
}))

describe('SipPage multi-account switch', () => {
  beforeEach(() => {
    localStorage.setItem('rex-token', 'tok')
    vi.clearAllMocks()
  })

  it('renders an account selector with all accounts and the active one selected', async () => {
    const wrapper = mount(SipPage, {
      props: { resourceId: 'r1', environmentId: 'env1', name: 'Phone' },
      global: { stubs: ['Dialpad', 'CallState'], plugins: [i18n] },
    })
    await flushPromises()
    await nextTick()

    const select = wrapper.find('.account-select')
    expect(select.exists()).toBe(true)
    const options = select.findAll('option').map((o) => o.text())
    expect(options).toEqual(['Alice', 'Bob'])
    expect(select.attributes('value')).toBe('a1')
  })

  it('switching account writes activeAccount back via update', async () => {
    const wrapper = mount(SipPage, {
      props: { resourceId: 'r1', environmentId: 'env1', name: 'Phone' },
      global: { stubs: ['Dialpad', 'CallState'], plugins: [i18n] },
    })
    await flushPromises()
    await nextTick()

    const select = wrapper.find('.account-select')
    await select.setValue('a2')
    await flushPromises()
    await nextTick()

    const mod = (await import('@/api/resources')) as any
    const resourcesApi = mod.resourcesApi
    expect(resourcesApi.update).toHaveBeenCalledTimes(1)
    const cfg = JSON.parse(resourcesApi.update.mock.calls[0]![2].config_json)
    expect(cfg.activeAccount).toBe('a2')
  })
})
