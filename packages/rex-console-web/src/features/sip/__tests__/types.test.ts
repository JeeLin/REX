import { describe, it, expect } from 'vitest'
import { resolveActiveAccount, type SipAccountView } from '../types'

const accounts: SipAccountView[] = [
  { id: 'a1', server: 'pbx1', port: 5060, transport: 'udp', username: 'alice' },
  { id: 'a2', server: 'pbx2', port: 5061, transport: 'tls', username: 'bob', displayName: 'Bob' },
]

describe('resolveActiveAccount', () => {
  it('returns the account matching activeAccount', () => {
    expect(resolveActiveAccount(accounts, 'a2')?.id).toBe('a2')
  })

  it('falls back to the first account when activeAccount is missing', () => {
    expect(resolveActiveAccount(accounts, 'does-not-exist')?.id).toBe('a1')
  })

  it('returns undefined for empty list', () => {
    expect(resolveActiveAccount([], 'a1')).toBeUndefined()
  })
})
