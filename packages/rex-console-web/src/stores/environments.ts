import { defineStore } from 'pinia'
import { ref } from 'vue'
import { environmentsApi, type Environment, type NewEnvironment } from '@/api/environments'
import { resourcesApi, type Resource, type NewResource, type TestConnectionRequest, type TestConnectionResult } from '@/api/resources'

export const useEnvironmentsStore = defineStore('environments', () => {
  const environments = ref<Environment[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  const envResources = ref<Map<string, Resource[]>>(new Map())

  // --- Environments ---

  async function fetchEnvironments() {
    loading.value = true
    error.value = null
    try {
      environments.value = await environmentsApi.list()
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      loading.value = false
    }
  }

  async function createEnvironment(data: NewEnvironment): Promise<Environment> {
    const env = await environmentsApi.create(data)
    environments.value.push(env)
    return env
  }

  async function updateEnvironment(id: string, data: Partial<NewEnvironment>): Promise<Environment> {
    const updated = await environmentsApi.update(id, data)
    const idx = environments.value.findIndex(e => e.id === id)
    if (idx >= 0) {
      environments.value[idx] = { ...environments.value[idx], ...updated }
    }
    return updated
  }

  async function deleteEnvironment(id: string) {
    await environmentsApi.delete(id)
    environments.value = environments.value.filter(e => e.id !== id)
  }

  // --- Resources ---

  async function fetchResources(envId: string): Promise<Resource[]> {
    const resources = await resourcesApi.listByEnv(envId)
    envResources.value.set(envId, resources)
    return resources
  }

  async function createResource(envId: string, data: NewResource): Promise<Resource> {
    const resource = await resourcesApi.create(envId, data)
    const list = envResources.value.get(envId) || []
    list.push(resource)
    envResources.value.set(envId, [...list])
    const env = environments.value.find(e => e.id === envId)
    if (env) env.resource_count++
    return resource
  }

  async function deleteResource(envId: string, id: string) {
    await resourcesApi.delete(envId, id)
    const list = envResources.value.get(envId)
    if (list) {
      envResources.value.set(envId, list.filter(r => r.id !== id))
    }
    const env = environments.value.find(e => e.id === envId)
    if (env && env.resource_count > 0) {
      env.resource_count--
    }
  }

  async function updateResource(envId: string, id: string, data: Partial<NewResource>): Promise<Resource> {
    const updated = await resourcesApi.update(envId, id, data)
    const list = envResources.value.get(envId)
    if (list) {
      const idx = list.findIndex(r => r.id === id)
      if (idx >= 0) {
        list[idx] = updated
        envResources.value.set(envId, [...list])
      }
    }
    return updated
  }

  async function testConnection(data: TestConnectionRequest): Promise<TestConnectionResult> {
    return resourcesApi.testConnection(data)
  }

  return {
    environments, loading, error, envResources,
    fetchEnvironments, createEnvironment, updateEnvironment, deleteEnvironment,
    fetchResources, createResource, updateResource, deleteResource, testConnection,
  }
})
