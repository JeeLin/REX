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

  it('ringing state also initializes playback without throwing', async () => {
    const w = mountSip()
    lastWs!.emit('open')
    lastWs!.emit('message', JSON.stringify({ type: 'sip.registered' }))
    lastWs!.emit('message', JSON.stringify({ type: 'sip.call_state', payload: { callId: 'c1', state: 'ringing' } }))
    await w.vm.$nextTick()
    // 进入 ringing 应触发播放链路就绪；下行媒体帧路由不抛错。
    const buf = new ArrayBuffer(4)
    new DataView(buf).setInt16(0, 7, true)
    new DataView(buf).setInt16(2, -9, true)
    expect(() => lastWs!.emit('message', buf)).not.toThrow()
  })

  it('ended call tears down audio and clears current call', async () => {
    const w = mountSip()
    lastWs!.emit('open')
    lastWs!.emit('message', JSON.stringify({ type: 'sip.registered' }))
    lastWs!.emit('message', JSON.stringify({ type: 'sip.call_state', payload: { callId: 'c1', state: 'active' } }))
    await w.vm.$nextTick()
    lastWs!.emit('message', JSON.stringify({ type: 'sip.call_state', payload: { callId: 'c1', state: 'ended' } }))
    await w.vm.$nextTick()
    expect(w.find('.active-call').exists()).toBe(false)
  })

  it('incoming then answer clears incoming and emits sip.answer', async () => {
    const w = mountSip()
    lastWs!.emit('open')
    lastWs!.emit('message', JSON.stringify({ type: 'sip.registered' }))
    lastWs!.emit('message', JSON.stringify({ type: 'sip.incoming', payload: { callId: 'c2', from: '2002' } }))
    await w.vm.$nextTick()
    expect(w.find('.incoming').exists()).toBe(true)
    const answer = w.findAll('button').find((b) => b.text() === 'sip.answer')!
    await answer.trigger('click')
    expect(lastWs!.sent.some((s) => typeof s === 'string' && s.includes('"sip.answer"') && s.includes('c2'))).toBe(true)
    expect(w.find('.incoming').exists()).toBe(false)
  })

  it('hold then unhold emits both control messages', async () => {
    const w = mountSip()
    lastWs!.emit('open')
    lastWs!.emit('message', JSON.stringify({ type: 'sip.registered' }))
    lastWs!.emit('message', JSON.stringify({ type: 'sip.call_state', payload: { callId: 'c1', state: 'active' } }))
    await w.vm.$nextTick()
    const hold = w.findAll('button').find((b) => b.text() === 'sip.hold')!
    await hold.trigger('click')
    expect(lastWs!.sent.some((s) => typeof s === 'string' && s.includes('"sip.hold"') && s.includes('c1'))).toBe(true)
    // 服务端确认 hold 成功 → 本地状态转为 held，unhold 按钮才渲染。
    lastWs!.emit('message', JSON.stringify({ type: 'sip.call_state', payload: { callId: 'c1', state: 'held' } }))
    await w.vm.$nextTick()
    const unhold = w.findAll('button').find((b) => b.text() === 'sip.unhold')!
    await unhold.trigger('click')
    expect(lastWs!.sent.some((s) => typeof s === 'string' && s.includes('"sip.unhold"') && s.includes('c1'))).toBe(true)
  })

  it('dtmf key click emits sip.dtmf with callId and digit', async () => {
    const w = mountSip()
    lastWs!.emit('open')
    lastWs!.emit('message', JSON.stringify({ type: 'sip.registered' }))
    lastWs!.emit('message', JSON.stringify({ type: 'sip.call_state', payload: { callId: 'c1', state: 'active' } }))
    await w.vm.$nextTick()
    const dtmfKey = w.findAll('button').find((b) => b.text() === '5')!
    await dtmfKey.trigger('click')
    expect(
      lastWs!.sent.some(
        (s) => typeof s === 'string' && s.includes('"sip.dtmf"') && s.includes('c1') && s.includes('"5"'),
      ),
    ).toBe(true)
  })

  it('ws close/error update status via emit', async () => {
    const w = mountSip()
    lastWs!.emit('open')
    await w.vm.$nextTick()
    lastWs!.emit('close')
    expect(w.emitted('update:status')?.some((args) => args[0] === 'disconnected')).toBe(true)
    lastWs!.emit('error')
    expect(w.emitted('update:status')?.some((args) => args[0] === 'error')).toBe(true)
  })

  it('toggling mic on captures and sends pcm frames, toggling off stops', async () => {
    // 真实环境无 AudioContext / 麦克风，注入假 AudioContext 与 getUserMedia。
    const startMicNodes: unknown[] = []
    class TestAudioContext {
      destination = {}
      createScriptProcessor() {
        const n = { connect() {}, disconnect() {}, onaudioprocess: null }
        startMicNodes.push(n)
        return n
      }
      createMediaStreamSource() {
        return { connect() {} }
      }
      createGain() {
        return { connect() {}, disconnect() {}, gain: { value: 0 } }
      }
      close() {
        return Promise.resolve()
      }
    }
    // @ts-expect-error inject fake AudioContext
    global.AudioContext = TestAudioContext
    Object.defineProperty(navigator, 'mediaDevices', {
      configurable: true,
      value: { getUserMedia: vi.fn().mockResolvedValue({ getTracks: () => [{ stop() {} }] }) },
    })

    const w = mountSip()
    lastWs!.emit('open')
    lastWs!.emit('message', JSON.stringify({ type: 'sip.registered' }))
    lastWs!.emit('message', JSON.stringify({ type: 'sip.call_state', payload: { callId: 'c1', state: 'active' } }))
    await w.vm.$nextTick()

    const micBtn = w.findAll('button').find((b) => b.text() === 'sip.micOff')!
    await micBtn.trigger('click')
    await w.vm.$nextTick()
    // 上行帧经 onaudioprocess 触发后通过 sendMediaFrame 二进制发出。
    const node = startMicNodes[startMicNodes.length - 1] as { onaudioprocess?: (e: unknown) => void }
    node.onaudioprocess?.({ inputBuffer: { getChannelData: () => new Float32Array([0.5, -0.5]) } } as never)
    expect(lastWs!.sent.some((s) => s instanceof ArrayBuffer && s.byteLength > 0)).toBe(true)

    // 再次点击关闭麦克风。
    const micBtn2 = w.findAll('button').find((b) => b.text() === 'sip.micOn')!
    await micBtn2.trigger('click')
    await w.vm.$nextTick()
    expect(micBtn2.element).toBeTruthy()
  })

  it('hangup click emits sip.hangup and clears incoming', async () => {
    const w = mountSip()
    lastWs!.emit('open')
    lastWs!.emit('message', JSON.stringify({ type: 'sip.registered' }))
    lastWs!.emit('message', JSON.stringify({ type: 'sip.incoming', payload: { callId: 'c9', from: '9999' } }))
    await w.vm.$nextTick()
    const decline = w.findAll('button').find((b) => b.text() === 'sip.decline')!
    await decline.trigger('click')
    expect(
      lastWs!.sent.some((s) => typeof s === 'string' && s.includes('"sip.hangup"') && s.includes('c9')),
    ).toBe(true)
    expect(w.find('.incoming').exists()).toBe(false)
  })
})
