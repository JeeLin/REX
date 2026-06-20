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

