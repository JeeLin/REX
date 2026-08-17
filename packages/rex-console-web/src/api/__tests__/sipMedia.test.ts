import { describe, it, expect } from 'vitest'
import {
  encodePcmFrame,
  decodeMediaFrame,
  SipAudio,
} from '../sipMedia'

// 浏览器环境无 AudioContext，SipAudio 应安全空转（supported=false）。
class FakeAudioContext {
  destination = {}
  createScriptProcessor() {
    return { connect() {}, disconnect() {}, onaudioprocess: null }
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

describe('sipMedia pcm encode/decode', () => {
  it('round-trips S16LE samples through ArrayBuffer', () => {
    const samples = new Int16Array([0, -32768, 32767, 1234, -4321, 7])
    const buf = encodePcmFrame(samples)
    expect(buf.byteLength).toBe(samples.length * 2)
    const decoded = decodeMediaFrame(buf)
    expect(Array.from(decoded)).toEqual(Array.from(samples))
  })

  it('decode drops trailing odd byte', () => {
    const samples = new Int16Array([10, -20])
    const buf = encodePcmFrame(samples)
    const odd = new Uint8Array(buf)
    const withTrailer = new Uint8Array([...odd, 0xab])
    const decoded = decodeMediaFrame(withTrailer.buffer)
    expect(Array.from(decoded)).toEqual([10, -20])
  })

  it('empty input decodes to empty', () => {
    expect(decodeMediaFrame(new ArrayBuffer(0)).length).toBe(0)
  })
})

describe('SipAudio unsafe env', () => {
  it('reports unsupported and no-ops without AudioContext', () => {
    const audio = new SipAudio()
    expect(audio.supported).toBe(false)
    // 下列调用在 unsupported 下不应抛错。
    audio.initPlayback()
    audio.playPcm(new Int16Array([1, 2, 3]))
    audio.stopMic()
    audio.close()
    expect(audio.micActive).toBe(false)
  })

  it('initPlayback no-ops when supported but disabled ctx ctor', () => {
    const audio = new SipAudio({ createAudioContext: FakeAudioContext as never })
    expect(audio.supported).toBe(true)
    audio.initPlayback()
    // 无麦克风时 playPcm 进队但不崩。
    audio.playPcm(new Int16Array([5, 6]))
    audio.close()
    expect(audio.micActive).toBe(false)
  })
})
