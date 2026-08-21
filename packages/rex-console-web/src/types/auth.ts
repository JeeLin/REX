export interface AuthCheckResponse {
  requires_setup: boolean
}

export interface LoginResponse {
  token: string
  expiresAt: string
}
