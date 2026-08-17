// SIP 媒体通道客户端（M82b）：浏览器侧处理原始 S16LE PCM 媒体帧。
//
// 线上约定与后端 `rex_common::sip_media` 完全一致——媒体帧即原始 S16LE（小端 i16）PCM
// 字节，不做 opus 等线上编码。下行（Hub→浏览器，远端 PCM）经 WebSocket 二进制帧推来，
// 浏览器用 Web Audio 原生播放；上行（浏览器→Hub，麦克风 PCM）经 getUserMedia 采集后
// 原样二进制帧回传。理由：单用户自托管局域网场景，Web Audio 原生消费/采集 PCM 即可，
// 省去 opus 依赖与每帧编解码延迟。

/** 媒体采样率（Hz），与 baresip 窄带语音一致。 */
export const MEDIA_SAMPLE_RATE = 8000
/** 媒体声道数（单声道）。 */
export const MEDIA_CHANNELS = 1
/** 单帧样本数（20ms @ 8kHz = 160）。 */
export const PCM_FRAME_SAMPLES = 160

/**
 * 将一帧 S16LE（小端 i16）PCM 样本编码为二进制帧字节（直接小端 i16 拼接）。
 * 与后端 `encode_pcm_frame` 对称。
 */
export function encodePcmFrame(samples: Int16Array): ArrayBuffer {
  const buf = new ArrayBuffer(samples.length * 2)
  const view = new DataView(buf)
  for (let i = 0; i < samples.length; i++) {
    view.setInt16(i * 2, samples[i] ?? 0, true)
  }
  return buf
}

/**
 * 将二进制帧字节解码为一帧 S16LE（小端 i16）PCM 样本。
 * 长度非偶数时丢弃末尾孤立字节（与后端 `decode_media_frame` 对称）。
 */
export function decodeMediaFrame(data: ArrayBuffer): Int16Array {
  const bytes = new Uint8Array(data)
  const n = Math.floor(bytes.length / 2)
  const out = new Int16Array(n)
  const view = new DataView(data)
  for (let i = 0; i < n; i++) {
    out[i] = view.getInt16(i * 2, true) ?? 0
  }
  return out
}

type AudioContextCtor = typeof AudioContext

/**
 * 浏览器实时音频端点：下行 PCM 播放 + 上行麦克风采集。
 *
 * 实现基于 Web Audio `ScriptProcessorNode`（无需额外 worklet 文件，兼容所有目标浏览器）。
 * 下行：把收到的 PCM 帧推进队列，回调中把队列样本灌入输出缓冲；上行：从麦克风捕获
 * 浮点样本转回 i16 经回调上抛。所有 Web Audio 操作在 `supported` 为 false 时安全空转
 *（测试/jsdom 环境无 AudioContext）。
 */
export class SipAudio {
  /** 运行环境是否具备 Web Audio 能力。 */
  readonly supported: boolean

  private readonly createCtx: AudioContextCtor | null
  private ctx: AudioContext | null = null
  private playNode: ScriptProcessorNode | null = null
  private playQueue: Int16Array[] = []
  private playOffset = 0
  private micStream: MediaStream | null = null
  private micNode: ScriptProcessorNode | null = null
  private micCallback: ((frame: Int16Array) => void) | null = null
  /** 是否正在上行采集麦克风。 */
  micActive = false

  constructor(opts?: { createAudioContext?: AudioContextCtor }) {
    const Ctor =
      opts?.createAudioContext ??
      (typeof AudioContext !== 'undefined' ? AudioContext : null) ??
      (typeof (window as unknown as { webkitAudioContext?: AudioContextCtor })
        .webkitAudioContext !== 'undefined'
        ? (window as unknown as { webkitAudioContext: AudioContextCtor })
            .webkitAudioContext
        : null)
    this.createCtx = Ctor
    this.supported = Ctor !== null
  }

  /** 初始化下行播放链路（AudioContext + ScriptProcessor 灌流到输出）。 */
  initPlayback(): void {
    if (!this.supported || !this.createCtx || this.ctx) return
    const ctx = new this.createCtx()
    this.ctx = ctx
    const node = ctx.createScriptProcessor(4096, 1, 1)
    node.onaudioprocess = (e: AudioProcessingEvent) => {
      const out = e.outputBuffer.getChannelData(0)
      let i = 0
      // 从队首帧（带已消费偏移 playOffset）顺序取样本填输出；队列不足处补静音。
      while (i < out.length) {
        if (this.playQueue.length === 0) break
        const frame = this.playQueue[0]
        if (!frame) break
        for (; this.playOffset < frame.length && i < out.length; this.playOffset++, i++) {
          out[i] = (frame[this.playOffset] ?? 0) / 32768
        }
        if (this.playOffset >= frame.length) {
          this.playQueue.shift()
          this.playOffset = 0
        }
      }
      for (; i < out.length; i++) out[i] = 0
    }
    node.connect(ctx.destination)
    this.playNode = node
  }

  /** 把一帧下行 PCM 样本推进播放队列。 */
  playPcm(frame: Int16Array): void {
    if (!this.playNode) return
    this.playQueue.push(frame)
  }

  /** 开始上行采集麦克风，每帧 PCM 经 `onFrame` 上抛。 */
  async startMic(onFrame: (frame: Int16Array) => void): Promise<void> {
    if (!this.supported || !this.createCtx || !this.ctx) return
    if (this.micActive) return
    const stream = await navigator.mediaDevices.getUserMedia({ audio: true })
    const source = this.ctx.createMediaStreamSource(stream)
    const node = this.ctx.createScriptProcessor(4096, 1, 1)
    const sink = this.ctx.createGain()
    sink.gain.value = 0 // 捕获路径不外放，避免回声
    node.onaudioprocess = (e: AudioProcessingEvent) => {
      const input = e.inputBuffer.getChannelData(0)
      const frame = new Int16Array(input.length)
      for (let i = 0; i < input.length; i++) {
        const s = Math.max(-1, Math.min(1, input[i] ?? 0))
        frame[i] = s < 0 ? s * 0x8000 : s * 0x7fff
      }
      onFrame(frame)
    }
    source.connect(node)
    node.connect(sink)
    sink.connect(this.ctx.destination)
    this.micStream = stream
    this.micNode = node
    this.micCallback = onFrame
    this.micActive = true
  }

  /** 停止上行采集。 */
  stopMic(): void {
    if (this.micNode) {
      this.micNode.disconnect()
      this.micNode = null
    }
    this.micStream?.getTracks().forEach((t) => t.stop())
    this.micStream = null
    this.micCallback = null
    this.micActive = false
  }

  /** 关闭全部音频（播放 + 采集 + AudioContext）。 */
  close(): void {
    this.stopMic()
    if (this.playNode) {
      this.playNode.disconnect()
      this.playNode = null
    }
    this.playQueue = []
    this.playOffset = 0
    if (this.ctx) {
      void this.ctx.close()
      this.ctx = null
    }
  }
}
