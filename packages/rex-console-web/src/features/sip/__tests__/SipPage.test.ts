import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import SipPage from '../SipPage.vue'

/* eslint-disable @typescript-eslint/no-this-alias */

vi.mock('vue-i18n', () => ({ useI18n: () => ({ t: (k: string) => k }) }))

// Capture the last constructed WebSocket so tests can drive events.
let lastWs: FakeWebSocket | null = null
class FakeWebSocket {
  static OPEN = 1
  readyState = FakeWebSocket.OPEN
  url = ''
  sent: (string | ArrayBuffer)[] = []
  onopen: (() => void) | null = null
  onmessage: ((ev: { data: string | ArrayBuffer }) => void) | null = null
  onclose: (() => void) | null = null
  onerror: ((e: Event) => void) | null = null
  constructor(url: string) {
    this.url = url
    lastWs = this
  }
  send(s: string | ArrayBuffer) {
    this.sent.push(s)
  }
  close() {}
  emit(type: string, data: string | ArrayBuffer = '') {
    if (type === 'open') this.onopen?.()
    else if (type === 'message') this.onmessage?.({ data })
    else if (type === 'close') this.onclose?.()
    else if (type === 'error') this.onerror?.(new Event('error'))
  }
}

beforeEach(() => {
  lastWs = null
  // @ts-expect-error override global WebSocket for the test
  global.WebSocket = FakeWebSocket
  localStorage.setItem('rex-token', 'tok123')
})

function mountSip() {
  return mount(SipPage, { props: { resourceId: 'res1', name: 'My Phone' } })
}

describe('SipPage event wiring', () => {
  it('connects to /ws/sip with token and resourceId', () => {
    mountSip()
    expect(lastWs).not.toBeNull()
    expect(lastWs!.url).toContain('/ws/sip')
    expect(lastWs!.url).toContain('resourceId=res1')
    expect(lastWs!.url).toContain('token=tok123')
  })

  it('enables dialing after sip.registered, dial sends sip.dial', async () => {
    const w = mountSip()
    lastWs!.emit('open')
    await w.vm.$nextTick()
    lastWs!.emit('message', JSON.stringify({ type: 'sip.registered' }))
    await w.vm.$nextTick()

    // call button stays disabled until a number is entered
    const callBtn = w.findAll('button').find((b) => b.text().includes('sip.call'))!
    await w.find('.dial-number').setValue('1000')
    await w.vm.$nextTick()
    expect((callBtn.element as HTMLButtonElement).disabled).toBe(false)

    await callBtn.trigger('click')
    expect(lastWs!.sent.some((s) => typeof s === 'string' && s.includes('"sip.dial"') && s.includes('1000'))).toBe(true)
  })

  it('sip.incoming renders answer popup and answer emits sip.answer', async () => {
    const w = mountSip()
    lastWs!.emit('open')
    lastWs!.emit('message', JSON.stringify({ type: 'sip.registered' }))
    lastWs!.emit('message', JSON.stringify({ type: 'sip.incoming', payload: { callId: 'c1', from: '1001' } }))
    await w.vm.$nextTick()

    expect(w.find('.incoming').exists()).toBe(true)
    const answer = w.findAll('button').find((b) => b.text() === 'sip.answer')!
    await answer.trigger('click')
    expect(lastWs!.sent.some((s) => typeof s === 'string' && s.includes('"sip.answer"') && s.includes('c1'))).toBe(true)
  })

  it('sip.call_state active drives state component and hold emits sip.hold', async () => {
    const w = mountSip()
    lastWs!.emit('open')
    lastWs!.emit('message', JSON.stringify({ type: 'sip.registered' }))
    lastWs!.emit('message', JSON.stringify({ type: 'sip.call_state', payload: { callId: 'c1', state: 'active' } }))
    await w.vm.$nextTick()

    expect(w.find('.active-call').exists()).toBe(true)
    const hold = w.findAll('button').find((b) => b.text() === 'sip.hold')!
    await hold.trigger('click')
    expect(lastWs!.sent.some((s) => typeof s === 'string' && s.includes('"sip.hold"') && s.includes('c1'))).toBe(true)
  })

  it('drives an active call into audio init and routes binary media frames', async () => {
    const w = mountSip()
    lastWs!.emit('open')
    lastWs!.emit('message', JSON.stringify({ type: 'sip.registered' }))
    lastWs!.emit('message', JSON.stringify({ type: 'sip.call_state', payload: { callId: 'c1', state: 'active' } }))
    await w.vm.$nextTick()

    // 下行二进制媒体帧（伪造 S16LE PCM）不应抛错（无 AudioContext 时安全空转）。
    const buf = new ArrayBuffer(4 * 2)
    new DataView(buf).setInt16(0, 1, true)
    new DataView(buf).setInt16(2, -2, true)
    new DataView(buf).setInt16(4, 3, true)
    new DataView(buf).setInt16(6, 4, true)
    expect(() => lastWs!.emit('message', buf)).not.toThrow()

    // 文本信令帧仍正常解析。
    expect(w.find('.active-call').exists()).toBe(true)
  })

  it('cleanup on unmount tears down audio without throwing', async () => {
    const w = mountSip()
    lastWs!.emit('open')
    await w.vm.$nextTick()
    expect(() => w.unmount()).not.toThrow()
  })
})
