import { describe, it, expect, vi, beforeEach } from 'vitest'

const { mockGet } = vi.hoisted(() => ({ mockGet: vi.fn() }))

vi.mock('../client', () => ({ api: { get: (...args: unknown[]) => mockGet(...args) } }))

import { auditApi, type AuditQuery } from '../audit'

function callAt(index: number): { path: string; params: Record<string, string> } {
  const call = mockGet.mock.calls[index] as [string, Record<string, string>]
  return { path: call[0], params: call[1] }
}

function lastQueryString(): string {
  return new URLSearchParams(callAt(mockGet.mock.calls.length - 1)!.params).toString()
}

const FULL_FILTERS: AuditQuery = {
  time_from: '2026-01-01T00:00:00Z',
  time_to: '2026-01-02T00:00:00Z',
  action: 'SSH_CONNECT',
  result: 'success',
  environment_id: 'env-1',
  resource_id: 'res-1',
  agent_id: 'ag-1',
}

beforeEach(() => {
  mockGet.mockReset()
  mockGet.mockResolvedValue([])
})

describe('auditApi.query', () => {
  it('forwards resource_id and agent_id', async () => {
    await auditApi.query({ resource_id: 'res-1', agent_id: 'ag-1' })

    const { path, params } = callAt(0)
    expect(path).toBe('/audit-log')
    expect(params.resource_id).toBe('res-1')
    expect(params.agent_id).toBe('ag-1')
  })

  it('serializes pagination alongside the new dimensions', async () => {
    await auditApi.query({ resource_id: 'res-1', agent_id: 'ag-1', limit: 50, offset: 100 })

    const params = callAt(0).params
    expect(params).toEqual({
      resource_id: 'res-1',
      agent_id: 'ag-1',
      limit: '50',
      offset: '100',
    })
  })

  it('omits dimensions that were not provided', async () => {
    await auditApi.query({ action: 'ENV_CREATE' })

    const params = callAt(0).params
    expect(params).toEqual({ action: 'ENV_CREATE' })
    expect(params).not.toHaveProperty('resource_id')
    expect(params).not.toHaveProperty('agent_id')
    expect(lastQueryString()).not.toContain('undefined')
  })

  it('omits every dimension when called with no params', async () => {
    await auditApi.query()

    expect(mockGet).toHaveBeenCalledWith('/audit-log', {})
    expect(lastQueryString()).toBe('')
  })
})

describe('auditApi.stats', () => {
  // Regression lock: filtering by resource/agent used to change the list but not the
  // statistics. stats() must forward the same dimensions or the counts silently lie.
  it('forwards resource_id and agent_id just like query()', async () => {
    await auditApi.stats({ resource_id: 'res-1', agent_id: 'ag-1' })

    const { path, params } = callAt(0)
    expect(path).toBe('/audit-log/stats')
    expect(params.resource_id).toBe('res-1')
    expect(params.agent_id).toBe('ag-1')
  })

  it('sends the identical filter dimensions as query()', async () => {
    await auditApi.query({ ...FULL_FILTERS, limit: 50, offset: 100 })
    await auditApi.stats({ ...FULL_FILTERS })

    // List and stats must be scoped identically, otherwise the header counts
    // silently describe a different slice of the log than the table below.
    const { limit: _l, offset: _o, ...listFilters } = callAt(0).params
    expect(callAt(1).params).toEqual(listFilters)
    expect(listFilters).toEqual({
      time_from: '2026-01-01T00:00:00Z',
      time_to: '2026-01-02T00:00:00Z',
      action: 'SSH_CONNECT',
      result: 'success',
      environment_id: 'env-1',
      resource_id: 'res-1',
      agent_id: 'ag-1',
    })
  })

  it('omits dimensions that were not provided', async () => {
    await auditApi.stats({ action: 'ENV_CREATE' })

    expect(callAt(0).params).toEqual({ action: 'ENV_CREATE' })
    expect(lastQueryString()).not.toContain('undefined')
  })

  it('never forwards pagination, even when it is passed at runtime', async () => {
    await auditApi.stats({ limit: 50, offset: 100 } as AuditQuery)

    const params = callAt(0).params
    expect(params).toEqual({})
    expect(params).not.toHaveProperty('limit')
    expect(params).not.toHaveProperty('offset')
  })
})
