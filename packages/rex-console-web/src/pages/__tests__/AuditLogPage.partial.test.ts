import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import type { AuditEntry, AuditQuery } from '@/api/audit'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    // Named placeholders render inline so assertions can see the substituted value
    t: (key: string, arg?: unknown) =>
      arg && typeof arg === 'object' ? `${key} ${JSON.stringify(arg)}` : key,
  }),
}))

const { mockQuery, mockStats } = vi.hoisted(() => ({ mockQuery: vi.fn(), mockStats: vi.fn() }))
const { mockListByEnv } = vi.hoisted(() => ({ mockListByEnv: vi.fn() }))
const { mockNotifyError, mockNotifyWarning } = vi.hoisted(() => ({
  mockNotifyError: vi.fn(),
  mockNotifyWarning: vi.fn(),
}))
const { envStore } = vi.hoisted(() => {
  const mockFetchEnvironments = vi.fn()
  const mockFetchResources = vi.fn()
  const store = {
    environments: [] as { id: string; name: string }[],
    envResources: new Map<string, { id: string; name: string }[]>(),
    fetchEnvironments: mockFetchEnvironments,
    fetchResources: mockFetchResources,
  }
  return { envStore: store }
})
const mockFetchEnvironments = vi.mocked(envStore.fetchEnvironments)
const mockFetchResources = vi.mocked(envStore.fetchResources)

vi.mock('@/api/audit', () => ({
  auditApi: {
    query: (...args: unknown[]) => mockQuery(...args),
    stats: (...args: unknown[]) => mockStats(...args),
  },
}))
vi.mock('@/api/agents', () => ({
  agentsApi: { listByEnv: (...args: unknown[]) => mockListByEnv(...args) },
}))
vi.mock('@/stores/environments', () => ({ useEnvironmentsStore: () => envStore }))
vi.mock('@/stores/notification', () => ({
  useNotificationStore: () => ({ error: mockNotifyError, warning: mockNotifyWarning }),
}))

import AuditLogPage from '../AuditLogPage.vue'

function entry(over: Partial<AuditEntry> = {}): AuditEntry {
  return {
    id: 'entry-1',
    time: '2026-01-01T10:00:00Z',
    action: 'SSH_CONNECT',
    target: 'web-1',
    environment_id: 'env-1',
    resource_id: 'res-1',
    agent_id: 'ag-1',
    result: 'success',
    detail: null,
    ...over,
  }
}

/** onMounted awaits environments, then per-env resources/agents, then the audit fetch. */
async function settle() {
  for (let i = 0; i < 5; i++) await flushPromises()
}

async function mountPage() {
  const wrapper = mount(AuditLogPage)
  await settle()
  return wrapper
}

function lastQueryParams(): Record<string, unknown> {
  return mockQuery.mock.calls[mockQuery.mock.calls.length - 1]![0] as Record<string, unknown>
}

function lastStatsParams(): Record<string, unknown> {
  return mockStats.mock.calls[mockStats.mock.calls.length - 1]![0] as Record<string, unknown>
}

async function clickChip(wrapper: ReturnType<typeof mount>, label: string) {
  const chip = wrapper.findAll('.filter-chip').find(c => c.text() === label)
  expect(chip, `chip "${label}" not rendered`).toBeTruthy()
  await chip!.trigger('click')
  await settle()
}

/** Footer controls: page total, [Prev, Next], page info, goto input. */
function footer(wrapper: ReturnType<typeof mount>) {
  const btns = wrapper.findAll('.page-btn')
  return {
    prev: btns[0]!.element as HTMLButtonElement,
    next: btns[1]!.element as HTMLButtonElement,
    info: wrapper.find('.page-info').text(),
    goto: wrapper.find('.page-goto-input').element as HTMLInputElement,
  }
}

beforeEach(() => {
  vi.clearAllMocks()
  mockFetchEnvironments.mockResolvedValue(undefined)
  mockFetchResources.mockResolvedValue([])
  mockListByEnv.mockResolvedValue([
    { id: 'ag-1', environment_id: 'env-1', name: 'agent-alpha' },
    { id: 'ag-2', environment_id: 'env-2', name: 'agent-beta' },
  ])
  envStore.environments = [{ id: 'env-1', name: 'prod' }]
  envStore.envResources = new Map([
    ['env-1', [{ id: 'res-1', name: 'redis-main' }, { id: 'res-2', name: 'pg-main' }]],
  ])
  mockQuery.mockResolvedValue([entry()])
  mockStats.mockResolvedValue({ total: 1, success_count: 1, failure_count: 0 })
})

describe('AuditLogPage partial failure', () => {
  it('keeps the list when only the statistics request fails, and shows — instead of 0', async () => {
    mockStats.mockRejectedValue(new Error('stats boom'))
    const wrapper = await mountPage()

    // List is still rendered
    expect(mockQuery).toHaveBeenCalled()
    expect(wrapper.findAll('tbody tr')).toHaveLength(1)

    // Stats show a failure marker, never a misleading 0
    const statValues = wrapper.findAll('.stat .stat-value')
    expect(statValues[0]!.text()).not.toBe('0')
    expect(statValues[1]!.text()).not.toBe('0')
    expect(statValues[2]!.text()).not.toBe('0')
    for (const v of statValues.slice(0, 3)) {
      expect(v.text()).toBe('—')
      expect(v.classes()).toContain('stat-value--error')
    }
    // Total is likewise unavailable, not 0
    expect(wrapper.find('.page-total').text()).toContain('—')
  })

  it('surfaces a readable warning when statistics fail', async () => {
    mockStats.mockRejectedValue(new Error('stats boom'))
    const wrapper = await mountPage()

    const note = wrapper.find('.load-note')
    expect(note.exists()).toBe(true)
    expect(note.text()).toContain('auditLog.statsFailed')
    // the raw backend error is shown, not swallowed
    expect(note.text()).toContain('stats boom')
    expect(mockNotifyWarning).toHaveBeenCalledWith('auditLog.statsFailed')
  })

  it('does not blank the table when both the list and the statistics fail', async () => {
    mockQuery.mockRejectedValue(new Error('list boom'))
    mockStats.mockRejectedValue(new Error('stats boom'))
    const wrapper = await mountPage()

    expect(wrapper.find('.load-note').text()).toContain('auditLog.loadFailed')
    expect(mockNotifyError).toHaveBeenCalledWith('auditLog.loadFailed')
    // Only one toast: the list already reported the failure, the stats warning
    // would otherwise be a second notification for the same refresh.
    expect(mockNotifyWarning).not.toHaveBeenCalled()
  })

  it('warns separately when the statistics fail after the list had succeeded', async () => {
    mockQuery.mockResolvedValue([entry()])
    mockStats.mockRejectedValue(new Error('stats boom'))
    const wrapper = await mountPage()
    expect(mockNotifyWarning).toHaveBeenCalledWith('auditLog.statsFailed')

    // Retry: the list succeeds again, the statistics keep failing.
    mockStats.mockRejectedValue(new Error('stats boom again'))
    const retryBtn = wrapper.find('.load-note').findAll('button').find(b => b.text().includes('common.refresh'))!
    await retryBtn.trigger('click')
    await settle()

    // No repeated toast for an unchanged failure, and the list stays rendered.
    expect(mockNotifyWarning).toHaveBeenCalledTimes(1)
    expect(wrapper.findAll('tbody tr')).toHaveLength(1)
  })

  it('shows the load failure instead of the empty state when the list fails', async () => {
    mockQuery.mockRejectedValue(new Error('list boom'))
    const wrapper = await mountPage()

    const empty = wrapper.find('.empty-state')
    expect(empty.exists()).toBe(true)
    expect(empty.text()).toContain('auditLog.loadFailed')
    expect(empty.text()).not.toContain('auditLog.noEntries')
    // the raw error reaches the user
    expect(empty.text()).toContain('list boom')
  })

  it('disables paging when the list fails so paging cannot fake an empty page', async () => {
    mockQuery.mockRejectedValue(new Error('list boom'))
    const wrapper = await mountPage()

    const f = footer(wrapper)
    expect(f.prev.disabled).toBe(true)
    expect(f.next.disabled).toBe(true)
    expect(f.goto.disabled).toBe(true)
  })

  it('never claims a page count when only the statistics fail', async () => {
    mockStats.mockRejectedValue(new Error('stats boom'))
    mockQuery.mockResolvedValue([entry()])
    const wrapper = await mountPage()

    // Total unknown: a "1 / 1" here would be a guess dressed up as a fact, and it
    // contradicts the "—" the total count shows right next to it.
    const f = footer(wrapper)
    expect(f.info).toBe('1 / —')
    expect(f.info).not.toBe('1 / 1')
    expect(f.info).not.toContain('/ 0')
    expect(wrapper.find('.page-total').text()).toContain('—')
    // Jumping needs a last page to clamp against, so it is disabled
    expect(f.goto.disabled).toBe(true)
  })

  it('keeps paging forward while the backend returns full pages, and stops at its end', async () => {
    mockStats.mockRejectedValue(new Error('stats boom'))
    const fullPage = Array.from({ length: 50 }, (_, i) => entry({ id: `entry-${i}` }))
    mockQuery.mockResolvedValue(fullPage)
    const wrapper = await mountPage()

    // Total unknown, but this page came back full: the list may continue
    let f = footer(wrapper)
    expect(f.info).toBe('1 / —')
    expect(f.next.disabled).toBe(false)

    await wrapper.findAll('.page-btn')[1]!.trigger('click')
    await settle()
    expect(lastQueryParams().offset).toBe(50)
    f = footer(wrapper)
    expect(f.info).toBe('2 / —')
    expect(f.prev.disabled).toBe(false)

    // A partial page is the end of the list: Next must not step past it
    mockQuery.mockResolvedValue([entry({ id: 'entry-last' })])
    await wrapper.findAll('.page-btn')[1]!.trigger('click')
    await settle()
    expect(lastQueryParams().offset).toBe(100)

    f = footer(wrapper)
    expect(f.info).toBe('3 / —')
    expect(f.next.disabled).toBe(true)
  })

  it('keeps the goto control usable and honest when the statistics succeed', async () => {
    mockQuery.mockResolvedValue([entry()])
    mockStats.mockResolvedValue({ total: 120, success_count: 100, failure_count: 20 })
    const wrapper = await mountPage()

    // 120 rows / 50 per page: the full pagination contract stays untouched
    const f = footer(wrapper)
    expect(f.info).toBe('1 / 3')
    expect(f.goto.disabled).toBe(false)
    expect(f.goto.max).toBe('3')
    expect(wrapper.find('.page-total').text()).toContain('120')
    expect(f.next.disabled).toBe(false)
    expect(f.prev.disabled).toBe(true)
  })

  it('still loads the audit list when resource/agent resolution fails', async () => {
    mockFetchResources.mockRejectedValue(new Error('resources boom'))
    mockListByEnv.mockRejectedValue(new Error('agents boom'))
    const wrapper = await mountPage()

    expect(mockQuery).toHaveBeenCalled()
    expect(mockStats).toHaveBeenCalled()
    expect(wrapper.findAll('tbody tr')).toHaveLength(1)
    expect(mockNotifyWarning).toHaveBeenCalledWith('auditLog.resourcesResolveFailed')
    expect(mockNotifyWarning).toHaveBeenCalledWith('auditLog.agentsResolveFailed')
    // Loading finished: the page is not stuck in the loading state
    expect(wrapper.find('.loading').exists()).toBe(false)
  })

  it('reports a CSV export failure instead of rejecting unhandled', async () => {
    mockQuery.mockRejectedValue(new Error('export boom'))
    const wrapper = await mountPage()

    const exportBtn = wrapper.findAll('button').find(b => b.text().includes('auditLog.exportCsv'))!
    await exportBtn.trigger('click')
    await settle()

    expect(mockNotifyError).toHaveBeenCalledWith('auditLog.exportFailed')
  })
})

describe('AuditLogPage shared filter scope', () => {
  it('sends the same filter dimensions to both the list and the statistics request', async () => {
    await mountPage()

    const listParams = lastQueryParams()
    const statsParams = lastStatsParams()
    // The list call adds pagination on top of the shared filter set
    const { limit: _limit, offset: _offset, ...filters } = listParams as unknown as AuditQuery & { limit: number; offset: number }
    expect(statsParams).toEqual(filters)
  })

  it('applies the resource filter to both the list and the statistics request', async () => {
    const wrapper = await mountPage()
    await clickChip(wrapper, 'redis-main')

    expect(lastQueryParams().resource_id).toBe('res-1')
    expect(lastStatsParams().resource_id).toBe('res-1')
  })

  it('applies the agent filter to both the list and the statistics request', async () => {
    const wrapper = await mountPage()
    await clickChip(wrapper, 'agent-alpha')

    expect(lastQueryParams().agent_id).toBe('ag-1')
    expect(lastStatsParams().agent_id).toBe('ag-1')
  })

  it('clears the resource and agent dimensions again when the filters are reset', async () => {
    const wrapper = await mountPage()
    await clickChip(wrapper, 'redis-main')
    await clickChip(wrapper, 'agent-alpha')

    const clearBtn = wrapper.findAll('button').find(b => b.text().includes('auditLog.clearFilters'))!
    await clearBtn.trigger('click')
    await settle()

    expect(lastQueryParams().resource_id).toBeUndefined()
    expect(lastStatsParams().resource_id).toBeUndefined()
    expect(lastQueryParams().agent_id).toBeUndefined()
    expect(lastStatsParams().agent_id).toBeUndefined()
  })

  it('includes the resource and agent dimensions in the CSV export', async () => {
    const wrapper = await mountPage()
    await clickChip(wrapper, 'redis-main')
    await clickChip(wrapper, 'agent-alpha')

    mockQuery.mockResolvedValue([entry()])
    const exportBtn = wrapper.findAll('button').find(b => b.text().includes('auditLog.exportCsv'))!
    await exportBtn.trigger('click')
    await settle()

    const exportParams = lastQueryParams()
    expect(exportParams.resource_id).toBe('res-1')
    expect(exportParams.agent_id).toBe('ag-1')
    expect(exportParams.limit).toBe(10000)
  })
})