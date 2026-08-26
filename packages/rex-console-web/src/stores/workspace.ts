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
  // v0.70.7：SQL 资源的子类（dialect）。经工作台 tab 透传给 SQL 控制台路由。
  subtype?: string
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
