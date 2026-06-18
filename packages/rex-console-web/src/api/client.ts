import axios from 'axios'
import router from '@/router'

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

// 响应拦截器：401 跳转登录
client.interceptors.response.use(
  (res) => res,
  (err) => {
    if (err.response?.status === 401) {
      localStorage.removeItem('rex-token')
      localStorage.removeItem('rex-expires-at')
      router.push('/login')
    }
    return Promise.reject(err)
  },
)

export default client
