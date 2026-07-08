import client from './client'

export interface Tag {
  id: string
  name: string
  color: string
  created_at: string
}

export async function listTags(): Promise<Tag[]> {
  const res = await client.get('/tags')
  return res.data.data
}

export async function createTag(data: { name: string; color?: string }): Promise<Tag> {
  const res = await client.post('/tags', data)
  return res.data.data
}

export async function updateTag(id: string, data: { name?: string; color?: string }): Promise<Tag> {
  const res = await client.put(`/tags/${id}`, data)
  return res.data.data
}

export async function deleteTag(id: string): Promise<void> {
  await client.delete(`/tags/${id}`)
}

export async function getResourceTags(resourceId: string): Promise<Tag[]> {
  const res = await client.get(`/resources/${resourceId}/tags`)
  return res.data.data
}

export async function setResourceTags(resourceId: string, tagIds: string[]): Promise<void> {
  await client.put(`/resources/${resourceId}/tags`, { tag_ids: tagIds })
}
