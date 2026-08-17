import { describe, it, expect } from 'vitest'
import { PROTOCOL_ICONS, PROTOCOL_COLORS, PROTOCOL_NAMES, type ProtocolType } from '../protocols'

describe('protocols registry', () => {
  it('includes sip with icon/color/name', () => {
    expect(PROTOCOL_ICONS.sip).toBeTruthy()
    expect(PROTOCOL_COLORS.sip).toMatch(/^#/)
    expect(PROTOCOL_NAMES.sip).toContain('SIP')
  })

  it('sip color is distinct from existing protocols', () => {
    const others = Object.entries(PROTOCOL_COLORS).filter(([k]) => k !== 'sip')
    expect(others.some(([, c]) => c === PROTOCOL_COLORS.sip)).toBe(false)
  })

  it('ProtocolType includes sip', () => {
    const t: ProtocolType = 'sip'
    expect(t).toBe('sip')
  })
})
