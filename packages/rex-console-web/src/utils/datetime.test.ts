import { describe, expect, it } from 'vitest'
import { formatDateTime } from './datetime'

describe('formatDateTime', () => {
  it('converts UTC to Shanghai time with an explicit offset', () => {
    const result = formatDateTime('2026-09-16T09:19:04.214Z', 'Asia/Shanghai')
    expect(result).toContain('17:19:04')
    expect(result).toContain('GMT+08:00')
  })
  it('does not double-convert an offset timestamp', () => {
    expect(formatDateTime('2026-09-16T17:19:04+08:00', 'Asia/Shanghai'))
      .toBe(formatDateTime('2026-09-16T09:19:04Z', 'Asia/Shanghai'))
  })
  it('respects daylight saving offsets', () => {
    expect(formatDateTime('2026-07-01T12:00:00Z', 'America/New_York')).toContain('GMT-04:00')
    expect(formatDateTime('2026-01-01T12:00:00Z', 'America/New_York')).toContain('GMT-05:00')
  })
  it('handles absent, invalid and timezone-less values without guessing', () => {
    expect(formatDateTime(null)).toBe('—')
    expect(formatDateTime('invalidZ')).toBe('invalidZ')
    expect(formatDateTime('2026-09-16 09:19:04')).toBe('2026-09-16 09:19:04')
  })
})
