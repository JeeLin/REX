import { describe, it, expect } from 'vitest'
import { getProtocolIcon } from '../useProtocol'

describe('getProtocolIcon', () => {
  it('returns correct icon for known protocols', () => {
    expect(getProtocolIcon('ssh')).toEqual({ icon: '$', color: '#22d3ee' })
    expect(getProtocolIcon('sftp')).toEqual({ icon: '📁', color: '#a78bfa' })
    expect(getProtocolIcon('mysql')).toEqual({ icon: 'dB', color: '#f59e0b' })
    expect(getProtocolIcon('postgresql')).toEqual({ icon: 'pg', color: '#60a5fa' })
    expect(getProtocolIcon('redis')).toEqual({ icon: 'r', color: '#ef4444' })
    expect(getProtocolIcon('docker')).toEqual({ icon: '🐳', color: '#3b82f6' })
    expect(getProtocolIcon('sqlite')).toEqual({ icon: 'db', color: '#10b981' })
    expect(getProtocolIcon('s3')).toEqual({ icon: '☁', color: '#f97316' })
  })

  it('returns fallback for unknown protocol', () => {
    expect(getProtocolIcon('unknown')).toEqual({ icon: '?', color: '#888' })
    expect(getProtocolIcon('')).toEqual({ icon: '?', color: '#888' })
  })
})
