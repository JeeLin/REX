import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { getHealth } from '@/api/health'

export const useAppStore = defineStore('app', () => {
  const mode = ref<'hub' | 'agent'>('hub')
  const version = ref('')
  const loading = ref(false)
  const error = ref<string | null>(null)

  const isHub = computed(() => mode.value === 'hub')
  const isAgent = computed(() => mode.value === 'agent')

  /** 检查连接模式（页面加载时调用） */
  async function checkMode() {
    loading.value = true
    error.value = null
    try {
      const res = await getHealth()
      mode.value = res.mode || 'hub'
      version.value = res.version || ''
    } catch (e) {
      // 如果健康检查失败，默认为 hub 模式
      mode.value = 'hub'
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      loading.value = false
    }
  }

  return {
    mode,
    version,
    loading,
    error,
    isHub,
    isAgent,
    checkMode,
  }
})