// SIP 配置共享类型与解析规则（0.70.5 抽取，消除三处镜像）。
//
// 后端 `rex_sip::SipProfile` / `SipAccount` 的字段对齐：
//   id / server / port / transport / username / password / displayName
// 生效账户解析规则与 `rex_hub::resource_conn::load_sip_conn` 一致：
// 取 activeAccount 对应账户，不存在则回退 accounts[0]。

export type SipTransport = 'udp' | 'tcp' | 'tls'

export interface SipAccountView {
  id: string
  server: string
  port: number
  transport: SipTransport
  username: string
  password?: string
  displayName?: string
}

// 向导内编辑态：port 可为空（未填），其余字段与 SipAccountView 一致。
export interface SipAccountForm {
  id: string
  server: string
  port: number | null
  transport: SipTransport
  username: string
  password: string
  displayName: string
}

export interface SipProfile {
  accounts: SipAccountView[]
  activeAccount: string
}

/**
 * 解析生效账户：优先 activeAccount 对应项，不存在则回退 accounts[0]。
 * 与后端 `load_sip_conn` 的 "active 或 first" 语义保持一致。
 */
export function resolveActiveAccount(accounts: SipAccountView[], activeAccount: string): SipAccountView | undefined {
  return accounts.find((a) => a.id === activeAccount) ?? accounts[0]
}
