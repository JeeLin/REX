import { describe, it, expect } from 'vitest'
import { encodeControl, decodeEvent, SipClient } from '@/api/sip'

describe('sip frame codec', () => {
  it('encodes dial with snake_case callId for answer? (no — control uses camlCase callId)', () => {
    const msg = encodeControl({ type: 'sip.dial', payload: { destination: 'sip:1000@example.com' } })
    expect(JSON.parse(msg)).toEqual({ type: 'sip.dial', payload: { destination: 'sip:1000@example.com' } })
  })

  it('encodes dtmf with callId camelCase', () => {
    const msg = encodeControl({ type: 'sip.dtmf', payload: { callId: 'c1', digit: '5' } })
    expect(JSON.parse(msg)).toMatchObject({ type: 'sip.dtmf', payload: { callId: 'c1', digit: '5' } })
  })

  it('decodes registered event', () => {
    const e = decodeEvent(JSON.stringify({ type: 'sip.registered' }))
    expect(e).toEqual({ type: 'sip.registered' })
  })

  it('decodes incoming with callId+from', () => {
    const e = decodeEvent(JSON.stringify({ type: 'sip.incoming', payload: { callId: 'c1', from: '1001' } }))
    expect(e).toEqual({ type: 'sip.incoming', payload: { callId: 'c1', from: '1001' } })
  })

  it('decodes call_state with state', () => {
    const e = decodeEvent(JSON.stringify({ type: 'sip.call_state', payload: { callId: 'c1', state: 'active' } }))
    expect(e).toEqual({ type: 'sip.call_state', payload: { callId: 'c1', state: 'active' } })
  })

  it('returns null on invalid json', () => {
    expect(decodeEvent('not-json')).toBeNull()
  })

  it('returns null when type missing', () => {
    expect(decodeEvent(JSON.stringify({ payload: {} }))).toBeNull()
  })
})

describe('SipClient send helpers', () => {
  it('dial/answer/hangup/hold/unhold/dtmf build correct messages over the socket', () => {
    const sent: string[] = []
    const client = new SipClient('res1')
    // bypass connect: inject a fake ws via the private field
    ;(client as unknown as { ws: WebSocket }).ws = {
      readyState: WebSocket.OPEN,
      send: (s: string) => sent.push(s),
    } as unknown as WebSocket

    client.dial('sip:1000@x.com')
    client.answer('c1')
    client.hangup('c1')
    client.hold('c1')
    client.unhold('c1')
    client.dtmf('c1', '#')

    expect(JSON.parse(sent[0]!)).toEqual({ type: 'sip.dial', payload: { destination: 'sip:1000@x.com' } })
    expect(JSON.parse(sent[1]!)).toEqual({ type: 'sip.answer', payload: { callId: 'c1' } })
    expect(JSON.parse(sent[2]!)).toEqual({ type: 'sip.hangup', payload: { callId: 'c1' } })
    expect(JSON.parse(sent[3]!)).toEqual({ type: 'sip.hold', payload: { callId: 'c1' } })
    expect(JSON.parse(sent[4]!)).toEqual({ type: 'sip.unhold', payload: { callId: 'c1' } })
    expect(JSON.parse(sent[5]!)).toEqual({ type: 'sip.dtmf', payload: { callId: 'c1', digit: '#' } })
  })
})
