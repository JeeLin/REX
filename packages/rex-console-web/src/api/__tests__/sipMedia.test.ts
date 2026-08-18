import { describe, it, expect, vi } from 'vitest'
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

  it('startMic captures float samples and converts to i16 frame', async () => {
    // 测试环境无真实媒体设备，mock getUserMedia 返回可停轨的假流。
    const getUserMedia = vi.fn().mockResolvedValue({
      getTracks: () => [{ stop() {} }],
    })
    Object.defineProperty(navigator, 'mediaDevices', {
      configurable: true,
      value: { getUserMedia },
    })
    const audio = new SipAudio({ createAudioContext: FakeAudioContext as never })
    audio.initPlayback()
    let captured: Int16Array | null = null
    await audio.startMic((frame) => {
      captured = frame
    })
    expect(audio.micActive).toBe(true)
    // 模拟 onaudioprocess：浮点输入（正/负/溢出钳制）应转为 i16 经回调上抛。
    const node = (audio as unknown as { micNode: { onaudioprocess?: (e: unknown) => void } }).micNode
    const input = new Float32Array([0.5, -0.25, 1.5, -1.5])
    let cb: ((e: { inputBuffer: { getChannelData: () => Float32Array } }) => void) | undefined
    // 直接调用节点回调（happy-dom 真实 onaudioprocess 不易触发，故手动注入）。
    node.onaudioprocess?.({ inputBuffer: { getChannelData: () => input } } as never)
    expect(captured).not.toBeNull()
    // 0.5 → 0.5*0x7fff 向零截断为 16383；-0.25 → -0.25*0x8000 为 -8192；1.5 / -1.5 钳制到 ±满量程。
    // （Int16Array 赋值对浮点向零截断，非四舍五入，故 16383.5 → 16383。）
    expect(captured![0]).toBe(Math.trunc(0.5 * 0x7fff))
    expect(captured![1]).toBe(Math.trunc(-0.25 * 0x8000))
    expect(captured![2]).toBe(0x7fff)
    expect(captured![3]).toBe(-0x8000)
    audio.stopMic()
    expect(audio.micActive).toBe(false)
  })

  it('close resets playback queue and offset', () => {
    const audio = new SipAudio({ createAudioContext: FakeAudioContext as never })
    audio.initPlayback()
    audio.playPcm(new Int16Array([1, 2, 3]))
    audio.close()
    // 关闭后再次播放不应抛错（队列已清空）。
    expect(() => audio.playPcm(new Int16Array([4, 5]))).not.toThrow()
    expect(audio.micActive).toBe(false)
  })

  it('playback onaudioprocess drains queued PCM into output buffer', () => {
    const audio = new SipAudio({ createAudioContext: FakeAudioContext as never })
    audio.initPlayback()
    audio.playPcm(new Int16Array([100, -200]))
    const node = (audio as unknown as { playNode: { onaudioprocess?: (e: unknown) => void } }).playNode
    expect(node).toBeTruthy()
    expect(typeof node.onaudioprocess).toBe('function')
    // 输出缓冲长度 2048（createScriptProcessor(2048,1,1)），灌入队列样本后补静音。
    const out = new Float32Array(2048)
    node.onaudioprocess?.({ outputBuffer: { getChannelData: () => out } } as never)
    expect(out[0]).toBeCloseTo(100 / 32768, 5)
    expect(out[1]).toBeCloseTo(-200 / 32768, 5)
    // 队列耗尽后其余采样补静音（0）。
    expect(out[2]).toBe(0)
  })
})
