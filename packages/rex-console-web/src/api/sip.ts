// SIP /ws/sip 客户端：消息帧编解码 + WebSocket 连接 + 心跳。
// 消息模型与后端 `crates/rex-hub/src/sip_ws.rs` 的 ClientMsg/ServerMsg 对齐。

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
  | { type: 'sip.error'; payload: { message: string } }
  | { type: 'sip.ping' }

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
  /** 下行媒体帧（原始 S16LE PCM 二进制）回调，仅媒体通道时触发。 */
  onMedia?: (data: ArrayBuffer) => void
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
      // 二进制帧 = 下行媒体（原始 S16LE PCM），直接交 onMedia 处理。
      if (typeof ev.data !== 'string') {
        if (ev.data instanceof ArrayBuffer) {
          this.handlers.onMedia?.(ev.data)
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

  /** 上行发送一帧媒体（原始 S16LE PCM 二进制帧）。 */
  sendMediaFrame(data: ArrayBuffer): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(data)
    }
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
