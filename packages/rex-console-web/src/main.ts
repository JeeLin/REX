import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import router from './router'
import { i18n } from './i18n'
import { handleApiError } from './api/client'
import './styles/tokens.css'
import './styles/global.css'

const app = createApp(App)
app.use(createPinia())
app.use(router)
app.use(i18n)

// 统一 REST 错误 toast：任何未被调用方捕获的 ApiError 自动弹 toast（带 code），不再吞成 console.error
window.addEventListener('unhandledrejection', (e) => {
  if (handleApiError(e.reason)) e.preventDefault()
})

app.mount('#app')
