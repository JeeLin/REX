import axios from 'axios'
import router from '@/router'
import { useToast } from '@/composables/useToast'

const client = axios.create({
  baseURL: '/api',
  timeout: 15_000,
  headers: { 'Content-Type': 'application/json' },
})

// 请求拦截器：注入 token
client.interceptors.request.use((config) => {
  const token = localStorage.getItem('rex-token')
  if (token) {
    config.headers.Authorization = `Bearer ${token}`
  }
  return config
})

// 响应拦截器：统一错误处理
client.interceptors.response.use(
  (res) => res,
  (err) => {
    const toast = useToast()

    // 401 跳转登录（不显示 Toast，避免重复提示）
    if (err.response?.status === 401) {
      localStorage.removeItem('rex-token')
      localStorage.removeItem('rex-expires-at')
      router.push('/login')
      return Promise.reject(err)
    }

    // 429 限流
    if (err.response?.status === 429) {
      toast.error(err.response?.data?.error?.message || '操作过于频繁，请稍后重试')
      return Promise.reject(err)
    }

    // 5xx 服务端错误
    if (err.response?.status >= 500) {
      toast.error(`服务器错误（${err.response.status}），请稍后重试`)
      return Promise.reject(err)
    }

    // 超时
    if (err.code === 'ECONNABORTED') {
      toast.error('请求超时，请稍后重试')
      return Promise.reject(err)
    }

    // 网络错误（无 response）
    if (!err.response) {
      toast.error('网络连接失败，请检查网络')
      return Promise.reject(err)
    }

    return Promise.reject(err)
  },
)

export default client
