import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import RedisStatus from '../RedisStatus.vue'
import type { RedisInfo } from '@/api/redis'

vi.mock('vue-i18n', () => ({ useI18n: () => ({ t: (k: string) => k }) }))

const mockGetInfo = vi.fn()
vi.mock('@/api/redis', () => ({
  getInfo: (...args: unknown[]) => mockGetInfo(...args),
}))

const mockInfo: RedisInfo = {
  redis_version: '7.2.4',
  os: 'Linux 6.1.0',
  process_id: '1234',
  connected_clients: '3',
  used_memory: '1.5M',
  used_memory_peak: '2.1M',
  total_commands_processed: '10240',
  keyspace: [
    { db: 'db0', keys: 100, expires: 10 },
    { db: 'db1', keys: 50, expires: 5 },
  ],
}

beforeEach(() => {
  vi.useFakeTimers()
  mockGetInfo.mockReset()
})
afterEach(() => { vi.useRealTimers() })

describe('RedisStatus', () => {
  it('shows loading text before info loads', () => {
    mockGetInfo.mockReturnValue(new Promise(() => {})) // never resolves
    const w = mount(RedisStatus, { props: { sessionId: 's1' } })
    expect(w.find('.status-loading').exists()).toBe(true)
  })

  it('renders status cards with server info after load', async () => {
    mockGetInfo.mockResolvedValue(mockInfo)
    const w = mount(RedisStatus, { props: { sessionId: 's1' } })

    await vi.advanceTimersByTimeAsync(0)
    await w.vm.$nextTick()

    const text = w.text()
    expect(text).toContain('redis.version')
    expect(text).toContain('7.2.4')
  })

  it('shows memory info in status cards', async () => {
    mockGetInfo.mockResolvedValue(mockInfo)
    const w = mount(RedisStatus, { props: { sessionId: 's1' } })

    await vi.advanceTimersByTimeAsync(0)
    await w.vm.$nextTick()

    const text = w.text()
    expect(text).toContain('1.5M')
    expect(text).toContain('2.1M')
  })

  it('renders keyspace table when data is present', async () => {
    mockGetInfo.mockResolvedValue(mockInfo)
    const w = mount(RedisStatus, { props: { sessionId: 's1' } })

    await vi.advanceTimersByTimeAsync(0)
    await w.vm.$nextTick()

    expect(w.find('.ks-table').exists()).toBe(true)
    const rows = w.findAll('.ks-table tbody tr')
    expect(rows).toHaveLength(2)
    expect(rows[0]!.text()).toContain('db0')
    expect(rows[1]!.text()).toContain('db1')
  })

  it('shows no-data text when info is null and not loading', async () => {
    mockGetInfo.mockResolvedValue(null)
    const w = mount(RedisStatus, { props: { sessionId: 's1' } })

    await vi.advanceTimersByTimeAsync(0)
    await w.vm.$nextTick()

    const loading = w.find('.status-loading')
    expect(loading.exists()).toBe(true)
    expect(loading.text()).toContain('redis.noData')
  })
})
