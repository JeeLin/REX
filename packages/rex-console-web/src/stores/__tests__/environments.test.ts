import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useEnvironmentsStore } from '../environments'

// Mock the API modules
vi.mock('@/api/environments', () => ({
  environmentsApi: {
    list: vi.fn(),
    create: vi.fn(),
    update: vi.fn(),
    delete: vi.fn(),
  },
}))

vi.mock('@/api/resources', () => ({
  resourcesApi: {
    listByEnv: vi.fn(),
    create: vi.fn(),
    delete: vi.fn(),
    testConnection: vi.fn(),
  },
}))

import { environmentsApi } from '@/api/environments'
import { resourcesApi } from '@/api/resources'
const mockEnvironmentsApi = vi.mocked(environmentsApi)
const mockResourcesApi = vi.mocked(resourcesApi)

describe('environments store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  describe('fetchEnvironments', () => {
    it('should load environments from API', async () => {
      const mockEnvs = [
        { id: '1', name: 'Production', description: '', connection_mode: 'direct', created_at: '', updated_at: '', resource_count: 0, agent_status: null },
        { id: '2', name: 'Staging', description: '', connection_mode: 'agent', created_at: '', updated_at: '', resource_count: 0, agent_status: null },
      ]
      mockEnvironmentsApi.list.mockResolvedValue(mockEnvs)
      const store = useEnvironmentsStore()
      await store.fetchEnvironments()
      expect(store.environments).toHaveLength(2)
      expect(store.loading).toBe(false)
    })

    it('should handle fetch error', async () => {
      mockEnvironmentsApi.list.mockRejectedValue(new Error('network error'))
      const store = useEnvironmentsStore()
      await store.fetchEnvironments()
      expect(store.error).toBe('network error')
      expect(store.loading).toBe(false)
    })
  })

  describe('createEnvironment', () => {
    it('should add new environment to list', async () => {
      const newEnv = { id: '3', name: 'New', description: '', connection_mode: 'direct', created_at: '', updated_at: '', resource_count: 0, agent_status: null }
      mockEnvironmentsApi.create.mockResolvedValue(newEnv)
      const store = useEnvironmentsStore()
      const result = await store.createEnvironment({ name: 'New', description: '', connection_mode: 'direct' })
      expect(result.id).toBe('3')
      expect(store.environments).toHaveLength(1)
      expect(store.environments[0]!.name).toBe('New')
    })
  })

  describe('updateEnvironment', () => {
    it('should update environment in list', async () => {
      const store = useEnvironmentsStore()
      store.environments = [
        { id: '1', name: 'Old', description: '', connection_mode: 'direct', created_at: '', updated_at: '', resource_count: 0, agent_status: null },
      ]
      mockEnvironmentsApi.update.mockResolvedValue({ id: '1', name: 'Updated', description: '', connection_mode: 'direct', created_at: '', updated_at: '', resource_count: 0, agent_status: null })
      await store.updateEnvironment('1', { name: 'Updated' })
      expect(store.environments[0]!.name).toBe('Updated')
    })
  })

  describe('deleteEnvironment', () => {
    it('should remove environment from list', async () => {
      const store = useEnvironmentsStore()
      store.environments = [
        { id: '1', name: 'ToDelete', description: '', connection_mode: 'direct', created_at: '', updated_at: '', resource_count: 0, agent_status: null },
      ]
      mockEnvironmentsApi.delete.mockResolvedValue({ ok: true })
      await store.deleteEnvironment('1')
      expect(store.environments).toHaveLength(0)
    })
  })

  describe('createResource', () => {
    it('should increment environment resource_count', async () => {
      const store = useEnvironmentsStore()
      store.environments = [
        { id: '1', name: 'Env', description: '', connection_mode: 'direct', created_at: '', updated_at: '', resource_count: 0, agent_status: null },
      ]
      const newRes = { id: 'r1', name: 'Res', protocol: 'ssh', host: 'localhost', environment_id: '1', port: 22, username: '', config_json: '{}', color: null, sort_order: 0, created_at: '', updated_at: '' }
      mockResourcesApi.create.mockResolvedValue(newRes)
      await store.createResource('1', { name: 'Res', protocol: 'ssh', host: 'localhost' })
      expect(store.environments[0]!.resource_count).toBe(1)
    })
  })

  describe('deleteResource', () => {
    it('should decrement environment resource_count', async () => {
      const store = useEnvironmentsStore()
      store.environments = [
        { id: '1', name: 'Env', description: '', connection_mode: 'direct', created_at: '', updated_at: '', resource_count: 2, agent_status: null },
      ]
      mockResourcesApi.delete.mockResolvedValue({ ok: true })
      await store.deleteResource('1', 'r1')
      expect(store.environments[0]!.resource_count).toBe(1)
    })
  })
})
