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
    update: vi.fn(),
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
        { id: '1', name: 'Production', description: '', connection_mode: 'direct', created_at: '', updated_at: '', resource_count: 0, agent_status: null, registration_token: 'test-token' },
        { id: '2', name: 'Staging', description: '', connection_mode: 'agent', created_at: '', updated_at: '', resource_count: 0, agent_status: null, registration_token: 'test-token' },
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
      const newEnv = { id: '3', name: 'New', description: '', connection_mode: 'direct', created_at: '', updated_at: '', resource_count: 0, agent_status: null, registration_token: 'test-token' }
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
        { id: '1', name: 'Old', description: '', connection_mode: 'direct', created_at: '', updated_at: '', resource_count: 0, agent_status: null, registration_token: 'test-token' },
      ]
      mockEnvironmentsApi.update.mockResolvedValue({ id: '1', name: 'Updated', description: '', connection_mode: 'direct', created_at: '', updated_at: '', resource_count: 0, agent_status: null, registration_token: 'test-token' })
      await store.updateEnvironment('1', { name: 'Updated' })
      expect(store.environments[0]!.name).toBe('Updated')
    })
  })

  describe('deleteEnvironment', () => {
    it('should remove environment from list', async () => {
      const store = useEnvironmentsStore()
      store.environments = [
        { id: '1', name: 'ToDelete', description: '', connection_mode: 'direct', created_at: '', updated_at: '', resource_count: 0, agent_status: null, registration_token: 'test-token' },
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
        { id: '1', name: 'Env', description: '', connection_mode: 'direct', created_at: '', updated_at: '', resource_count: 0, agent_status: null, registration_token: 'test-token' },
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
        { id: '1', name: 'Env', description: '', connection_mode: 'direct', created_at: '', updated_at: '', resource_count: 2, agent_status: null, registration_token: 'test-token' },
      ]
      mockResourcesApi.delete.mockResolvedValue({ ok: true })
      await store.deleteResource('1', 'r1')
      expect(store.environments[0]!.resource_count).toBe(1)
    })

    it('should not decrement below zero', async () => {
      const store = useEnvironmentsStore()
      store.environments = [
        { id: '1', name: 'Env', description: '', connection_mode: 'direct', created_at: '', updated_at: '', resource_count: 0, agent_status: null, registration_token: 'test-token' },
      ]
      mockResourcesApi.delete.mockResolvedValue({ ok: true })
      await store.deleteResource('1', 'r1')
      expect(store.environments[0]!.resource_count).toBe(0)
    })

    it('should not throw when env not found', async () => {
      const store = useEnvironmentsStore()
      mockResourcesApi.delete.mockResolvedValue({ ok: true })
      await expect(store.deleteResource('missing', 'r1')).resolves.toBeUndefined()
    })
  })

  describe('fetchResources', () => {
    it('should load resources into envResources map', async () => {
      const store = useEnvironmentsStore()
      const resources = [
        { id: 'r1', name: 'Res', protocol: 'ssh', host: 'localhost', environment_id: '1', port: 22, username: '', config_json: '{}', color: null, sort_order: 0, created_at: '', updated_at: '' },
      ]
      mockResourcesApi.listByEnv.mockResolvedValue(resources)
      const result = await store.fetchResources('1')
      expect(result).toHaveLength(1)
      expect(store.envResources.get('1')).toHaveLength(1)
    })
  })

  describe('updateResource', () => {
    it('should update resource in cache map', async () => {
      const store = useEnvironmentsStore()
      store.envResources.set('1', [
        { id: 'r1', name: 'Old', protocol: 'ssh', host: 'localhost', environment_id: '1', port: 22, username: '', config_json: '{}', color: null, sort_order: 0, created_at: '', updated_at: '' },
      ])
      const updated = { id: 'r1', name: 'New', protocol: 'ssh', host: 'localhost', environment_id: '1', port: 22, username: '', config_json: '{}', color: null, sort_order: 0, created_at: '', updated_at: '' }
      mockResourcesApi.update.mockResolvedValue(updated)
      await store.updateResource('1', 'r1', { name: 'New', protocol: 'ssh', host: 'localhost' })
      expect(store.envResources.get('1')![0]!.name).toBe('New')
    })

    it('should be a no-op when env not cached', async () => {
      const store = useEnvironmentsStore()
      mockResourcesApi.update.mockResolvedValue({ id: 'r1', name: 'New', protocol: 'ssh', host: 'localhost', environment_id: '1', port: 22, username: '', config_json: '{}', color: null, sort_order: 0, created_at: '', updated_at: '' })
      await expect(store.updateResource('x', 'r1', { name: 'New', protocol: 'ssh', host: 'localhost' })).resolves.toBeDefined()
    })
  })

  describe('createResource', () => {
    it('should push to envResources even when env list not cached', async () => {
      const store = useEnvironmentsStore()
      const newRes = { id: 'r1', name: 'Res', protocol: 'ssh', host: 'localhost', environment_id: '1', port: 22, username: '', config_json: '{}', color: null, sort_order: 0, created_at: '', updated_at: '' }
      mockResourcesApi.create.mockResolvedValue(newRes)
      await store.createResource('1', { name: 'Res', protocol: 'ssh', host: 'localhost' })
      expect(store.envResources.get('1')).toHaveLength(1)
    })

    it('should not increment env count when env not found', async () => {
      const store = useEnvironmentsStore()
      mockResourcesApi.create.mockResolvedValue({ id: 'r1' } as never)
      await expect(store.createResource('x', { name: 'Res', protocol: 'ssh', host: 'localhost' } as never)).resolves.toBeDefined()
    })
  })

  describe('testConnection', () => {
    it('should delegate to resourcesApi', async () => {
      const store = useEnvironmentsStore()
      mockResourcesApi.testConnection.mockResolvedValue({ ok: true, latency_ms: 12 })
      const result = await store.testConnection({ resource_id: 'r1' } as never)
      expect(result.ok).toBe(true)
    })
  })
})
