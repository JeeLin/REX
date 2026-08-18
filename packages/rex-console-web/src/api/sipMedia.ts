// SIP 媒体通道客户端（M82b 音频 + 0.70.2 子任务 #1 视频）：浏览器侧处理原始媒体字节。
//
// 线上约定与后端 `rex_common::sip_media` 完全一致——媒体帧即原始字节，不做线上编码：
// - 音频：S16LE（小端 i16）PCM 字节；下行经 Web Audio 播放，上行经 getUserMedia 采集回传。
// - 视频：RGBA 像素字节（每像素 4 字节，行优先）；下行经 Canvas 渲染，上行经 getUserMedia
//   视频轨采集回传。
// 隧道二进制帧首字节为 kind：`KIND_MEDIA`(1)=音频 PCM，`KIND_VIDEO`(2)=视频像素（子任务 #1）。
// 理由：单用户自托管局域网场景，Web Audio / Canvas 原生消费采集原始字节即可，省去编解码依赖。

/** 媒体采样率（Hz），与 baresip 窄带语音一致。 */
export const MEDIA_SAMPLE_RATE = 8000
/** 媒体声道数（单声道）。 */
export const MEDIA_CHANNELS = 1
/** 单帧样本数（20ms @ 8kHz = 160）。 */
export const PCM_FRAME_SAMPLES = 160

/** 隧道二进制帧 kind —— 与后端 `rex_common::sip_media::KIND_*` 对齐。 */
export const KIND_SIGNAL = 0
/** kind=1：音频媒体帧（原始 S16LE PCM 字节）。 */
export const KIND_MEDIA = 1
/** kind=2：视频媒体帧（原始 RGBA 像素字节，子任务 #1），payload=`[fmt:u8][w:u16][h:u16][rgba]`。 */
export const KIND_VIDEO = 2

/** 视频像素格式：RGBA（每像素 4 字节，行优先），与后端 `VideoPixFmt::Rgba` 对齐。 */
export const VIDEO_PIXFMT_RGBA = 0
/** RGBA 每像素字节数。 */
export const VIDEO_BPP = 4

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

/**
 * 将一帧视频像素（RGBA 行优先）编码为隧道视频帧字节：
 * `[fmt:u8][w:u16 LE][h:u16 LE][rgba...]`。
 * 与后端 `encode_video_frame` 对称。像素长度必须 = w*h*4，否则抛错。
 */
export function encodeVideoFrame(
  width: number,
  height: number,
  rgba: Uint8Array | Uint8ClampedArray,
): ArrayBuffer {
  const need = width * height * VIDEO_BPP
  if (rgba.length !== need) {
    throw new Error(`video pixel buffer size mismatch: got ${rgba.length}, need ${need}`)
  }
  const buf = new ArrayBuffer(5 + rgba.length)
  const view = new DataView(buf)
  view.setUint8(0, VIDEO_PIXFMT_RGBA)
  view.setUint16(1, width, true)
  view.setUint16(3, height, true)
  new Uint8Array(buf, 5).set(rgba)
  return buf
}

/**
 * 解码隧道视频帧字节为 `(fmt, width, height, rgba)`。长度不足或 fmt 未知抛错。
 * 与后端 `decode_video_frame` 对称。
 */
export function decodeVideoFrame(
  data: ArrayBuffer,
): { fmt: number; width: number; height: number; rgba: Uint8Array } {
  const bytes = new Uint8Array(data)
  if (bytes.length < 5) throw new Error('video frame too short')
  const fmt = bytes[0]
  if (fmt !== VIDEO_PIXFMT_RGBA) throw new Error('unknown video pixfmt')
  const width = (bytes[1]! | (bytes[2]! << 8)) >>> 0
  const height = (bytes[3]! | (bytes[4]! << 8)) >>> 0
  const rgba = bytes.subarray(5)
  const need = width * height * VIDEO_BPP
  if (rgba.length !== need) throw new Error('video pixel buffer size mismatch')
  return { fmt, width, height, rgba }
}

/** 用 RGBA 像素填充 Canvas 2D 上下文（行优先，BPP=4）。供视频下行渲染使用。 */
export function paintRgbaToCanvas(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  rgba: Uint8Array | Uint8ClampedArray,
): void {
  const img = new ImageData(width, height)
  img.data.set(rgba as Uint8ClampedArray)
  ctx.putImageData(img, 0, 0)
}

type VideoFrameCallback = (
  frame: { width: number; height: number; rgba: Uint8Array | Uint8ClampedArray },
) => void

/**
 * 浏览器实时视频端点（子任务 #1）：下行 RGBA 像素帧渲染到 `<canvas>` + 上行摄像头采集回传。
 *
 * 下行：把收到的视频帧像素直接 `putImageData` 到 canvas（无需 WebCodecs/解码器，原始 RGBA
 * 即渲染就绪）。上行：从摄像头 getUserMedia 取视频轨，逐帧 `drawImage` 到离屏 canvas 取回
 * RGBA 像素经回调上抛（与音频麦克风采集同构）。
 *
 * 所有媒体 API 操作在 `supported` 为 false 时安全空转（测试/jsdom 环境无相关 API）。
 */
export class SipVideo {
  /** 运行环境是否具备 Canvas 2D 能力。 */
  readonly supported: boolean

  private readonly createCanvas: (() => HTMLCanvasElement) | null
  private canvas: HTMLCanvasElement | null = null
  private ctx: CanvasRenderingContext2D | null = null
  private camStream: MediaStream | null = null
  private camRaf: number | null = null
  private offscreen: HTMLCanvasElement | null = null
  private offctx: CanvasRenderingContext2D | null = null
  private frameCallback: VideoFrameCallback | null = null
  /** 是否正在上行采集摄像头。 */
  camActive = false

  constructor(opts?: {
    createCanvas?: () => HTMLCanvasElement
    document?: Document
  }) {
    const doc =
      opts?.document ??
      (typeof document !== 'undefined' ? document : null)
    const ctor = opts?.createCanvas
    this.createCanvas = ctor ?? (doc ? () => doc.createElement('canvas') : null)
    this.supported = this.createCanvas !== null
  }

  /** 绑定一个用于下行渲染的 canvas 元素（调用方在模板中声明）。 */
  attachCanvas(canvas: HTMLCanvasElement): void {
    this.canvas = canvas
    this.ctx = canvas.getContext('2d')
  }

  /** 渲染一帧下行视频像素（RGBA 行优先）到已绑定的 canvas。 */
  renderFrame(width: number, height: number, rgba: Uint8Array): void {
    if (!this.ctx) return
    if (this.canvas && (this.canvas.width !== width || this.canvas.height !== height)) {
      this.canvas.width = width
      this.canvas.height = height
    }
    paintRgbaToCanvas(this.ctx, width, height, rgba)
  }

  /** 开始上行采集摄像头，每帧 RGBA 像素经 `onFrame` 上抛。 */
  async startCamera(onFrame: VideoFrameCallback): Promise<void> {
    if (!this.supported || this.camActive) return
    if (!this.createCanvas) return
    const stream = await navigator.mediaDevices.getUserMedia({ video: true })
    const video = document.createElement('video')
    video.srcObject = stream
    video.muted = true
    await video.play().catch(() => {})
    const off = this.createCanvas()
    off.width = video.videoWidth || 320
    off.height = video.videoHeight || 240
    const offctx = off.getContext('2d')
    if (!offctx) {
      stream.getTracks().forEach((t) => t.stop())
      return
    }
    this.camStream = stream
    this.offscreen = off
    this.offctx = offctx
    this.frameCallback = onFrame
    const loop = () => {
      if (!this.camStream || !this.offctx) return
      const w = this.offscreen!.width
      const h = this.offscreen!.height
      this.offctx.drawImage(video, 0, 0, w, h)
      const rgba = this.offctx.getImageData(0, 0, w, h).data
      this.frameCallback?.({ width: w, height: h, rgba })
      this.camRaf = requestAnimationFrame(loop)
    }
    this.camRaf = requestAnimationFrame(loop)
    this.camActive = true
  }

  /** 停止上行采集。 */
  stopCamera(): void {
    if (this.camRaf !== null) {
      cancelAnimationFrame(this.camRaf)
      this.camRaf = null
    }
    this.camStream?.getTracks().forEach((t) => t.stop())
    this.camStream = null
    this.offscreen = null
    this.offctx = null
    this.frameCallback = null
    this.camActive = false
  }

  /** 关闭全部视频（canvas 绑定 + 摄像头采集）。 */
  close(): void {
    this.stopCamera()
    this.canvas = null
    this.ctx = null
  }
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
    const node = ctx.createScriptProcessor(2048, 1, 1)
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
    const node = this.ctx.createScriptProcessor(2048, 1, 1)
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
