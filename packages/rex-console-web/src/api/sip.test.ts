import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { cdrApi, type CdrRecord } from './sip'

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
    const url = fetchMock.mock.calls[0][0] as string
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
    const url = fetchMock.mock.calls[0][0] as string
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
    expect(fetchMock.mock.calls[0][0]).toBe('/api/sip/cdr/cdr:call-1')
  })

  it('list throws ApiError on non-2xx', async () => {
    mockFetchOnce({ error: { code: 'X', message: 'boom' } }, 500)
    await expect(cdrApi.list()).rejects.toThrow()
  })
})
