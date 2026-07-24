import { defineStore } from 'pinia'
import { ref } from 'vue'

export interface ResourceInfo {
  id: string
  name: string
  protocol: string
  host?: string
  port?: number
  username?: string
  environmentId?: string
  color?: string
}

export const useWorkspaceStore = defineStore('workspace', () => {
  /** 从侧栏点击资源时设置，WorkspacePage 消费后清空 */
  const pendingResource = ref<ResourceInfo | null>(null)

  function openResource(resource: ResourceInfo) {
    pendingResource.value = resource
  }

  function consumePending(): ResourceInfo | null {
    const r = pendingResource.value
    pendingResource.value = null
    return r
  }

  return { pendingResource, openResource, consumePending }
})
