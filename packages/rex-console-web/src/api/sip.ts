// SIP /ws/sip 客户端：消息帧编解码 + WebSocket 连接 + 心跳。
// 消息模型与后端 `crates/rex-hub/src/sip_ws.rs` 的 ClientMsg/ServerMsg 对齐。

import { api } from './client'

export type SipCallState = 'ringing' | 'active' | 'held' | 'ended'

// 浏览器 → Hub 的控制指令
export type SipClientMsg =
  | { type: 'sip.dial'; payload: { destination: string } }
  | { type: 'sip.answer'; payload: { callId: string } }
  | { type: 'sip.hangup'; payload: { callId: string } }
  | { type: 'sip.hold'; payload: { callId: string } }
  | { type: 'sip.unhold'; payload: { callId: string } }
  | { type: 'sip.dtmf'; payload: { callId: string; digit: string } }
  | { type: 'ping' }

// Hub → 浏览器的事件
export type SipServerEvent =
  | { type: 'sip.registered' }
  | { type: 'sip.registration_failed'; payload: { reason: string } }
  | { type: 'sip.incoming'; payload: { callId: string; from: string } }
  | { type: 'sip.call_state'; payload: { callId: string; state: SipCallState } }
  | { type: 'sip.sip_message'; payload: { raw: string } }
  | { type: 'sip.quality'; payload: { loss: number; jitter: number; rtt: number } }
  | { type: 'sip.error'; payload: { message: string } }
  | { type: 'sip.ping' }

// --- CDR（通话记录）API 类型（与 /api/sip/cdr 对齐）---

export type CdrDirection = 'out' | 'in'
export type CdrState = 'ringing' | 'active' | 'held' | 'ended' | 'missed'

export interface CdrRecord {
  id: string
  resource_id: string
  peer: string
  call_id: string
  start_time: string
  end_time: string | null
  duration_sec: number
  direction: CdrDirection
  state: CdrState
  recording_url: string
  pcap_url: string
}

export interface CdrQuery {
  resource_id?: string
  direction?: CdrDirection
  state?: CdrState
  from?: string
  to?: string
  sort?: 'start_desc' | 'start_asc'
  limit?: number
  offset?: number
}

export interface CdrListResult {
  records: CdrRecord[]
  total: number
}

export const cdrApi = {
  list: (params?: CdrQuery) => {
    const query: Record<string, string> = {}
    if (params?.resource_id) query.resourceId = params.resource_id
    if (params?.direction) query.direction = params.direction
    if (params?.state) query.state = params.state
    if (params?.from) query.from = params.from
    if (params?.to) query.to = params.to
    if (params?.sort) query.sort = params.sort
    if (params?.limit) query.limit = String(params.limit)
    if (params?.offset) query.offset = String(params.offset)
    return api.get<CdrListResult>('/sip/cdr', query)
  },
  get: (id: string) => api.get<CdrRecord>(`/sip/cdr/${id}`),
}

// --- SIP 信令抓包 API（与 /api/sip/capture 对齐）---

export interface SipCaptureRecord {
  ts_us: number
  direction: 'ua1_out' | 'ua1_in' | 'ua2_in'
  raw: string
}

export const sipCaptureApi = {
  /** 开始对某 resource 抓包（幂等）。 */
  start: (resourceId: string) =>
    api.post<{ resource_id: string; active: boolean }>(
      `/sip/capture/${encodeURIComponent(resourceId)}/start`,
    ),
  /** 停止抓包，返回累计报文数。 */
  stop: (resourceId: string) =>
    api.post<{ resource_id: string; active: boolean; count: number }>(
      `/sip/capture/${encodeURIComponent(resourceId)}/stop`,
    ),
  /** 取当前累积的报文快照（可分页）。 */
  packets: (resourceId: string, limit?: number) => {
    const query: Record<string, string> = {}
    if (limit) query.limit = String(limit)
    return api.get<SipCaptureRecord[]>(
      `/sip/capture/${encodeURIComponent(resourceId)}/packets`,
      query,
    )
  },
  /** 下载 pcap 文件（链路类型 RAW，包体为 SIP 文本）。供 `<a href>` 直链下载。 */
  pcapUrl: (resourceId: string) =>
    `/api/sip/capture/${encodeURIComponent(resourceId)}/pcap`,
}

// --- SIP 通话录音开关（与 /api/sip/recording 对齐，子任务 #2）---

export const recordingApi = {
  /** 开始录音（幂等，按 resource 全局开启；按 call_id 分文件落盘）。 */
  start: (resourceId: string) =>
    api.post<{ resource_id: string; active: boolean }>(
      `/sip/recording/${encodeURIComponent(resourceId)}/start`,
    ),
  /** 停止录音。 */
  stop: (resourceId: string) =>
    api.post<{ resource_id: string; active: boolean }>(
      `/sip/recording/${encodeURIComponent(resourceId)}/stop`,
    ),
}

export function encodeControl(msg: SipClientMsg): string {
  return JSON.stringify(msg)
}

export function decodeEvent(raw: string): SipServerEvent | null {
  try {
    const parsed = JSON.parse(raw) as SipServerEvent
    if (typeof parsed?.type !== 'string') return null
    return parsed
  } catch {
    return null
  }
}

export type SipClientHandlers = {
  onEvent?: (e: SipServerEvent) => void
  /** 下行音频媒体帧（原始 S16LE PCM 二进制，kind=1）回调，仅媒体通道时触发。 */
  onMedia?: (data: ArrayBuffer) => void
  /** 下行视频媒体帧（原始 RGBA 像素二进制，kind=2）回调，仅媒体通道时触发（子任务 #1）。 */
  onVideo?: (data: ArrayBuffer) => void
  onOpen?: () => void
  onClose?: () => void
  onError?: (e: Event) => void
}

export class SipClient {
  private ws: WebSocket | null = null
  private heartbeat: ReturnType<typeof setInterval> | null = null

  constructor(
    private readonly resourceId: string,
    private readonly handlers: SipClientHandlers = {},
  ) {}

  connect(token: string): void {
    const proto = location.protocol === 'https:' ? 'wss' : 'ws'
    const url = `${proto}//${location.host}/ws/sip?resourceId=${encodeURIComponent(this.resourceId)}&token=${encodeURIComponent(token)}`
    const ws = new WebSocket(url)
    // 媒体帧为二进制（ArrayBuffer），需显式声明 binaryType 才能拿到 ArrayBuffer。
    ws.binaryType = 'arraybuffer'
    this.ws = ws
    ws.onopen = () => {
      this.startHeartbeat()
      this.handlers.onOpen?.()
    }
    ws.onmessage = (ev) => {
      // 二进制帧 = 下行媒体（首字节 kind 区分音频 PCM / 视频像素），拆 kind 后分派。
      if (typeof ev.data !== 'string') {
        if (ev.data instanceof ArrayBuffer) {
          const bytes = new Uint8Array(ev.data)
          const kind = bytes[0] ?? 0
          // 剥去首字节 kind，把 payload 交对应回调。
          const payload = ev.data.slice(1)
          if (kind === 2) {
            this.handlers.onVideo?.(payload)
          } else {
            // kind=1（音频）及其它默认按音频媒体处理。
            this.handlers.onMedia?.(payload)
          }
        }
        return
      }
      const event = decodeEvent(ev.data)
      if (event) this.handlers.onEvent?.(event)
    }
    ws.onclose = () => {
      this.stopHeartbeat()
      this.handlers.onClose?.()
    }
    ws.onerror = (e) => this.handlers.onError?.(e)
  }

  /** 上行发送一帧音频媒体（原始 S16LE PCM 二进制帧）。 */
  sendMediaFrame(data: ArrayBuffer): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(data)
    }
  }

  /**
   * 上行发送一帧视频媒体（原始 RGBA 像素二进制帧）。包裹 kind=2 首字节后发送，
   * 与后端 `KIND_VIDEO` 路由对齐（子任务 #1）。
   */
  sendVideoFrame(data: ArrayBuffer): void {
    if (this.ws?.readyState !== WebSocket.OPEN) return
    const payload = new Uint8Array(data)
    const framed = new Uint8Array(payload.length + 1)
    framed[0] = 2 // KIND_VIDEO
    framed.set(payload, 1)
    this.ws.send(framed.buffer)
  }

  private startHeartbeat(): void {
    this.stopHeartbeat()
    this.heartbeat = setInterval(() => this.send({ type: 'ping' }), 30_000)
  }

  private stopHeartbeat(): void {
    if (this.heartbeat !== null) {
      clearInterval(this.heartbeat)
      this.heartbeat = null
    }
  }

  send(msg: SipClientMsg): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(encodeControl(msg))
    }
  }

  dial(destination: string): void {
    this.send({ type: 'sip.dial', payload: { destination } })
  }

  answer(callId: string): void {
    this.send({ type: 'sip.answer', payload: { callId } })
  }

  hangup(callId: string): void {
    this.send({ type: 'sip.hangup', payload: { callId } })
  }

  hold(callId: string): void {
    this.send({ type: 'sip.hold', payload: { callId } })
  }

  unhold(callId: string): void {
    this.send({ type: 'sip.unhold', payload: { callId } })
  }

  dtmf(callId: string, digit: string): void {
    this.send({ type: 'sip.dtmf', payload: { callId, digit } })
  }

  close(): void {
    this.stopHeartbeat()
    this.ws?.close()
    this.ws = null
  }
}
