import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import WizardModal from '../WizardModal.vue'

// i18n stub: returns the key itself.
vi.mock('vue-i18n', () => ({
  useI18n: () => ({ t: (k: string) => k }),
}))

// Stub Modal（默认 Teleport 到 body，组件测试里改为直接渲染 slot，避免 teleport 干扰）。
// 用纯对象组件（无需 import），工厂会在顶部提升，外部 import 此时尚未初始化。
vi.mock('@/components/ui/Modal.vue', () => ({
  default: { name: 'ModalStub', template: '<div class="modal-stub"><slot /></div>' },
}))

// 工厂内部缓存同一实例，保证组件调用与测试断言拿到的是同一个 store 对象。
let storeInstance: { createResource: ReturnType<typeof vi.fn>; testConnection: ReturnType<typeof vi.fn> } | null = null
vi.mock('@/stores/environments', () => ({
  useEnvironmentsStore: () => {
    if (!storeInstance) {
      storeInstance = {
        createResource: vi.fn().mockResolvedValue({}),
        testConnection: vi.fn().mockResolvedValue({ ok: true, latency_ms: 10 }),
      }
    }
    return storeInstance
  },
}))

import { useEnvironmentsStore } from '@/stores/environments'
const mockStore = useEnvironmentsStore()

const tick = () => new Promise((r) => setTimeout(r, 0))

function clickSip(wrapper: ReturnType<typeof mount>) {
  const sip = wrapper.findAll('.protocol-card').find((b) => b.text().toLowerCase().includes('sip'))
  expect(sip).toBeTruthy()
  return sip!.trigger('click')
}

describe('WizardModal SIP config section', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('renders a SIP protocol card', () => {
    const wrapper = mount(WizardModal, {
      props: { visible: true, environmentId: 'env1' },
    })
    expect(wrapper.findAll('.protocol-card').some((b) => b.text().toLowerCase().includes('sip'))).toBe(true)
  })

  it('shows SIP form fields after selecting sip protocol', async () => {
    const wrapper = mount(WizardModal, {
      props: { visible: true, environmentId: 'env1' },
    })
    await clickSip(wrapper)
    await wrapper.vm.$nextTick()

    expect(wrapper.text()).toContain('wizard.sipServer')
    const select = wrapper.find('select.form-input')
    expect(select.exists()).toBe(true)
    const options = select.findAll('option').map((o) => o.attributes('value'))
    expect(options).toEqual(expect.arrayContaining(['udp', 'tcp', 'tls']))
  })

  it('serializes sip config_json with server/transport on submit', async () => {
    const wrapper = mount(WizardModal, {
      props: { visible: true, environmentId: 'env1' },
    })
    await clickSip(wrapper)
    await wrapper.vm.$nextTick()

    await wrapper.find('input[placeholder="e.g. sip.example.com"]').setValue('sip.example.com')
    await wrapper.find('input[placeholder="e.g. 1000"]').setValue('1000')
    await wrapper.find('select.form-input').setValue('tls')
    await wrapper.find('input[placeholder="e.g. Web Server"]').setValue('My Phone')

    const submitBtn = wrapper.findAll('button').find((b) => b.text().includes('wizard.create'))
    expect(submitBtn).toBeTruthy()
    await submitBtn!.trigger('click')
    await wrapper.vm.$nextTick()
    await tick()

    expect(mockStore.createResource).toHaveBeenCalledTimes(1)
    const call = (mockStore.createResource as unknown as ReturnType<typeof vi.fn>).mock.calls[0]!
    const payload = call[1] as { protocol: string; host: string; config_json: string }
    expect(payload.protocol).toBe('sip')
    const cfg = JSON.parse(payload.config_json)
    expect(cfg.server).toBe('sip.example.com')
    expect(cfg.transport).toBe('tls')
    expect(payload.host).toBe('sip.example.com')
  })
})
