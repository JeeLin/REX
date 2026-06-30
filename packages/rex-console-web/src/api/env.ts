import client from './client'

export interface Environment {
  id: string
  name: string
  description: string | null
  connection_mode: string
  created_at: string
  updated_at: string
}

export interface Resource {
  id: string
  environment_id: string
  name: string
  protocol: string
  config_json: string
  status: string
  created_at: string
  updated_at: string
}

export interface EnvWithResources extends Environment {
  resources: Resource[]
}

export async function listEnvironments(): Promise<Environment[]> {
  const res = await client.get('/environments')
  return res.data.data
}

export async function getEnvironment(id: string): Promise<Environment> {
  const res = await client.get(`/environments/${id}`)
  return res.data.data
}

export async function listResources(envId: string): Promise<Resource[]> {
  const res = await client.get(`/environments/${envId}/resources`)
  return res.data.data
}

export async function getResource(envId: string, id: string): Promise<Resource> {
  const res = await client.get(`/environments/${envId}/resources/${id}`)
  return res.data.data
}

export async function updateResource(envId: string, id: string, data: { name: string; config_json: string }): Promise<Resource> {
  const res = await client.put(`/environments/${envId}/resources/${id}`, data)
  return res.data.data
}

/** Delete a resource from an environment */
export async function deleteResource(envId: string, id: string): Promise<void> {
  await client.delete(`/environments/${envId}/resources/${id}`)
}

/** Update environment name / description */
export async function updateEnvironment(
  id: string,
  data: { name: string; description: string | null },
): Promise<Environment> {
  const res = await client.put(`/environments/${id}`, data)
  return res.data.data
}

/** Delete environment (cascades to its resources) */
export async function deleteEnvironment(id: string): Promise<void> {
  await client.delete(`/environments/${id}`)
}

/** Ping a resource to check its connection status */
export async function pingResource(
  envId: string,
  resourceId: string,
): Promise<{ status: string; latency_ms: number }> {
  const res = await client.post(`/environments/${envId}/resources/${resourceId}/ping`)
  return res.data.data
}

/** Fetch all environments with their resources for sidebar tree */
export async function listEnvsWithResources(): Promise<EnvWithResources[]> {
  const envs = await listEnvironments()
  const results = await Promise.all(
    envs.map(async (env) => {
      const resources = await listResources(env.id).catch(() => [] as Resource[])
      return { ...env, resources }
    }),
  )
  return results
}

