import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import {
  cdrApi,
  sipCaptureApi,
  recordingApi,
  SipClient,
  type CdrRecord,
} from './sip'

// 用可控的假 WebSocket 驱动 SipClient 的 onmessage 路由（验证 kind 分流）。
class FakeWebSocket {
  static OPEN = 1
  readyState = FakeWebSocket.OPEN
  onopen: ((ev: unknown) => void) | null = null
  onmessage: ((ev: { data: string | ArrayBuffer }) => void) | null = null
  onclose: (() => void) | null = null
  onerror: ((e: Event) => void) | null = null
  sent: (string | ArrayBuffer)[] = []
  constructor(_url: string) {
    queueMicrotask(() => this.onopen?.({}))
  }
  send(data: string | ArrayBuffer) {
    this.sent.push(data)
  }
  close() {
    this.onclose?.()
  }
}

// 用 fetch mock 验证 CDR API 客户端参数拼装与响应解析（不依赖后端）。
function mockFetchOnce(body: unknown, status = 200) {
  const fetchMock = vi.fn().mockResolvedValue({
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
  })
  vi.stubGlobal('fetch', fetchMock)
  return fetchMock
}

const sampleRecord: CdrRecord = {
  id: 'cdr:call-1',
  resource_id: 'res-1',
  peer: 'sip:bob@x',
  call_id: 'call-1',
  start_time: '2026-08-18T10:00:00Z',
  end_time: '2026-08-18T10:05:00Z',
  duration_sec: 300,
  direction: 'out',
  state: 'ended',
  recording_url: '/files/call-1.mp3',
  pcap_url: '',
}

describe('cdrApi', () => {
  beforeEach(() => {
    localStorage.setItem('rex-token', 'tok')
  })
  afterEach(() => {
    vi.unstubAllGlobals()
    vi.restoreAllMocks()
  })

  it('list builds query string from params', async () => {
    const fetchMock = mockFetchOnce({
      records: [sampleRecord],
      total: 1,
    })
    const res = await cdrApi.list({
      direction: 'out',
      state: 'ended',
      resource_id: 'res-1',
      sort: 'start_desc',
      limit: 20,
      offset: 40,
    })
    expect(res.records).toHaveLength(1)
    expect(res.total).toBe(1)
    const url = fetchMock.mock.calls[0]![0] as string
    expect(url).toContain('/api/sip/cdr?')
    expect(url).toContain('direction=out')
    expect(url).toContain('state=ended')
    expect(url).toContain('resourceId=res-1')
    expect(url).toContain('sort=start_desc')
    expect(url).toContain('limit=20')
    expect(url).toContain('offset=40')
  })

  it('list omits empty params', async () => {
    const fetchMock = mockFetchOnce({ records: [], total: 0 })
    await cdrApi.list(undefined)
    const url = fetchMock.mock.calls[0]![0] as string
    // 无过滤参数时不应携带任何 query key（仅可能带空 ? 后缀）。
    expect(url).not.toContain('direction=')
    expect(url).not.toContain('state=')
    expect(url).not.toContain('resourceId=')
    expect(url.startsWith('/api/sip/cdr')).toBe(true)
  })

  it('get fetches single record by id', async () => {
    const fetchMock = mockFetchOnce(sampleRecord)
    const rec = await cdrApi.get('cdr:call-1')
    expect(rec.id).toBe('cdr:call-1')
    expect(fetchMock.mock.calls[0]![0]).toBe('/api/sip/cdr/cdr:call-1')
  })

  it('list throws ApiError on non-2xx', async () => {
    mockFetchOnce({ error: { code: 'X', message: 'boom' } }, 500)
    await expect(cdrApi.list()).rejects.toThrow()
  })
})

describe('sipCaptureApi', () => {
  beforeEach(() => {
    localStorage.setItem('rex-token', 'tok')
  })
  afterEach(() => {
    vi.unstubAllGlobals()
    vi.restoreAllMocks()
  })

  it('start posts to capture start endpoint', async () => {
    const fetchMock = mockFetchOnce({ resource_id: 'res-1', active: true })
    const r = await sipCaptureApi.start('res-1')
    expect(r.active).toBe(true)
    expect(fetchMock.mock.calls[0]![0]).toBe('/api/sip/capture/res-1/start')
    expect(fetchMock.mock.calls[0]![1]).toMatchObject({ method: 'POST' })
  })

  it('stop posts and returns count', async () => {
    const fetchMock = mockFetchOnce({ resource_id: 'res-1', active: false, count: 3 })
    const r = await sipCaptureApi.stop('res-1')
    expect(r.count).toBe(3)
    expect(r.active).toBe(false)
    expect(fetchMock.mock.calls[0]![0]).toBe('/api/sip/capture/res-1/stop')
  })

  it('packets fetches snapshot with optional limit', async () => {
    const fetchMock = mockFetchOnce([])
    await sipCaptureApi.packets('res-1', 50)
    const url = fetchMock.mock.calls[0]![0] as string
    expect(url).toContain('/api/sip/capture/res-1/packets?')
    expect(url).toContain('limit=50')
  })

  it('pcapUrl builds download path', () => {
    expect(sipCaptureApi.pcapUrl('res-1')).toBe('/api/sip/capture/res-1/pcap')
  })
})

describe('recordingApi', () => {
  beforeEach(() => {
    localStorage.setItem('rex-token', 'tok')
  })
  afterEach(() => {
    vi.unstubAllGlobals()
    vi.restoreAllMocks()
  })

  it('start posts to recording start endpoint', async () => {
    const fetchMock = mockFetchOnce({ resource_id: 'res-1', active: true })
    const r = await recordingApi.start('res-1')
    expect(r.active).toBe(true)
    expect(fetchMock.mock.calls[0]![0]).toBe('/api/sip/recording/res-1/start')
    expect(fetchMock.mock.calls[0]![1]).toMatchObject({ method: 'POST' })
  })

  it('stop posts to recording stop endpoint', async () => {
    const fetchMock = mockFetchOnce({ resource_id: 'res-1', active: false })
    const r = await recordingApi.stop('res-1')
    expect(r.active).toBe(false)
    expect(fetchMock.mock.calls[0]![0]).toBe('/api/sip/recording/res-1/stop')
  })
})

describe('SipClient binary frame kind demux', () => {
  beforeEach(() => {
    vi.stubGlobal('WebSocket', FakeWebSocket)
    vi.stubGlobal('location', { protocol: 'http:', host: 'localhost' })
  })
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('routes kind=2 video frame to onVideo, not onMedia', () => {
    const onMedia = vi.fn()
    const onVideo = vi.fn()
    const client = new SipClient('r1', { onMedia, onVideo })
    client.connect('tok')
    const ws = (client as unknown as { ws: FakeWebSocket }).ws
    // 完整下行帧：首字节 kind=2 + 视频像素（fmt=0,w=2,h=2,4*4=16 rgba），共 22 字节。
    const pix = new Uint8Array([0, 2, 0, 2, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16])
    const frame = new Uint8Array(1 + pix.length)
    frame[0] = 2 // KIND_VIDEO
    frame.set(pix, 1)
    ws.onmessage?.({ data: frame.buffer })
    expect(onVideo).toHaveBeenCalledOnce()
    expect(onMedia).not.toHaveBeenCalled()
    // onVideo 收到的已是剥去 kind 的 payload。
    const got = new Uint8Array(onVideo.mock.calls[0]![0] as ArrayBuffer)
    expect(got[0]).toBe(0) // fmt
    expect(got.length).toBe(21) // 5 头 + 16 像素
  })

  it('routes kind=1 audio frame to onMedia, not onVideo', () => {
    const onMedia = vi.fn()
    const onVideo = vi.fn()
    const client = new SipClient('r1', { onMedia, onVideo })
    client.connect('tok')
    const ws = (client as unknown as { ws: FakeWebSocket }).ws
    const pcm = new Int16Array([1, -2, 3])
    const bytes = new Uint8Array(1 + pcm.length * 2)
    bytes[0] = 1 // KIND_MEDIA
    const view = new DataView(bytes.buffer)
    pcm.forEach((s, i) => view.setInt16(1 + i * 2, s, true))
    ws.onmessage?.({ data: bytes.buffer })
    expect(onMedia).toHaveBeenCalledOnce()
    expect(onVideo).not.toHaveBeenCalled()
  })

  it('sendVideoFrame wraps payload with kind=2', () => {
    const client = new SipClient('r1', {})
    client.connect('tok')
    const ws = (client as unknown as { ws: FakeWebSocket }).ws
    // 视频像素帧（已编码，无 kind）：fmt=0,w=1,h=1,4 rgba。
    const pix = new Uint8Array([0, 1, 0, 1, 9, 8, 7, 6])
    client.sendVideoFrame(pix.buffer)
    const sent = new Uint8Array(ws.sent[0] as ArrayBuffer)
    expect(sent[0]).toBe(2) // KIND_VIDEO
    expect(Array.from(sent.subarray(1))).toEqual(Array.from(pix))
  })
})

