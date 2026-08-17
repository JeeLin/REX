import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import CallState from '../CallState.vue'

vi.mock('vue-i18n', () => ({ useI18n: () => ({ t: (k: string) => k }) }))

describe('CallState', () => {
  it('shows registered status and no call UI when registered only', () => {
    const w = mount(CallState, {
      props: { registered: true, incoming: null, call: null },
    })
    expect(w.text()).toContain('sip.registered')
    expect(w.find('.incoming').exists()).toBe(false)
    expect(w.find('.active-call').exists()).toBe(false)
  })

  it('renders answer/decline buttons on incoming call', async () => {
    const w = mount(CallState, {
      props: { registered: true, incoming: { callId: 'c1', from: '1001' }, call: null },
    })
    expect(w.find('.incoming').exists()).toBe(true)
    const answer = w.findAll('button').find((b) => b.text() === 'sip.answer')!
    const decline = w.findAll('button').find((b) => b.text() === 'sip.decline')!
    expect(answer.exists()).toBe(true)
    expect(decline.exists()).toBe(true)
    await answer.trigger('click')
    expect(w.emitted('answer')?.[0]).toEqual(['c1'])
    await decline.trigger('click')
    expect(w.emitted('hangup')?.[0]).toEqual(['c1'])
  })

  it('renders active call with hold + dtmf, emits on interaction', async () => {
    const w = mount(CallState, {
      props: {
        registered: true,
        incoming: null,
        call: { callId: 'c1', state: 'active', from: '1001' },
      },
    })
    expect(w.find('.active-call').exists()).toBe(true)
    const hold = w.findAll('button').find((b) => b.text() === 'sip.hold')!
    await hold.trigger('click')
    expect(w.emitted('hold')?.[0]).toEqual(['c1'])

    const dtmfKey = w.findAll('.dtmf-key').find((b) => b.text() === '5')!
    await dtmfKey.trigger('click')
    expect(w.emitted('dtmf')?.[0]).toEqual(['c1', '5'])

    const hangup = w.findAll('button').find((b) => b.text() === 'sip.hangup')!
    await hangup.trigger('click')
    expect(w.emitted('hangup')?.[0]).toEqual(['c1'])
  })

  it('renders unhold button when call is held', () => {
    const w = mount(CallState, {
      props: {
        registered: true,
        incoming: null,
        call: { callId: 'c1', state: 'held', from: '1001' },
      },
    })
    const unhold = w.findAll('button').find((b) => b.text() === 'sip.unhold')!
    expect(unhold.exists()).toBe(true)
  })
})
