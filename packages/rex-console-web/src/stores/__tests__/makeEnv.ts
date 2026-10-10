import type { Environment } from '@/api/environments'

/**
 * Shared `Environment` fixture for the store and page suites. Consumers only
 * move these objects around or render them, so each case overrides just what it
 * cares about; adding a field to `Environment` then means editing this one file
 * instead of every literal in the suite.
 *
 * The default is the *maximal* environment — `connection_mode: 'agent'` so the
 * agent sections render. A case that means to exercise the direct-mode branch
 * passes `connection_mode: 'direct'` explicitly, which reads better than having
 * the base fixture hide the agent UI behind a quiet default. `id: '1'` is
 * likewise the default the store suite addresses resources by.
 */
export function makeEnv(over: Partial<Environment> = {}): Environment {
  return {
    id: '1',
    name: 'Env',
    description: '',
    connection_mode: 'agent',
    created_at: '',
    updated_at: '',
    resource_count: 0,
    agent_status: null,
    agents_online: 0,
    registration_token: 'test-token',
    ...over,
  }
}