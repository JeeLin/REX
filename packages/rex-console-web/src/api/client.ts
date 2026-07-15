//! 统一 HTTP API 客户端，自动注入 auth header、处理 401、统一错误格式。

export class AuthError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'AuthError'
  }
}

export class ApiError extends Error {
  code: string
  constructor(code: string, message: string) {
    super(message)
    this.name = 'ApiError'
    this.code = code
  }
}

class ApiClient {
  private baseUrl = '/api'

  private getHeaders(isFormData = false): Record<string, string> {
    const headers: Record<string, string> = {}
    if (!isFormData) {
      headers['Content-Type'] = 'application/json'
    }
    const token = localStorage.getItem('rex-token')
    if (token) {
      headers['Authorization'] = `Bearer ${token}`
    }
    return headers
  }

  async request<T>(path: string, options: RequestInit = {}): Promise<T> {
    const res = await fetch(`${this.baseUrl}${path}`, {
      ...options,
      headers: {
        ...this.getHeaders(),
        ...options.headers as Record<string, string>,
      },
    })

    if (res.status === 401) {
      localStorage.removeItem('rex-token')
      throw new AuthError('认证已过期')
    }

    if (!res.ok) {
      const body = await res.json().catch(() => null)
      throw new ApiError(
        body?.error?.code || 'UNKNOWN',
        body?.error?.message || res.statusText,
      )
    }

    return res.json()
  }

  get<T>(path: string, params?: Record<string, string>): Promise<T> {
    const url = params
      ? `${path}?${new URLSearchParams(params).toString()}`
      : path
    return this.request<T>(url)
  }

  post<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>(path, {
      method: 'POST',
      body: body ? JSON.stringify(body) : undefined,
    })
  }

  put<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>(path, {
      method: 'PUT',
      body: body ? JSON.stringify(body) : undefined,
    })
  }

  del<T>(path: string): Promise<T> {
    return this.request<T>(path, { method: 'DELETE' })
  }

  async upload<T>(path: string, formData: FormData): Promise<T> {
    return this.request<T>(path, {
      method: 'POST',
      body: formData,
      headers: this.getHeaders(true),
    })
  }
}

export const api = new ApiClient()
