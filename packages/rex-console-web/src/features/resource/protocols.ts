/** 协议图标、颜色、名称映射 */
export const PROTOCOL_ICONS: Record<string, string> = {
  ssh: '$',
  sftp: '📁',
  // v0.70.7：mysql/postgresql/sqlite 合并为单一「SQL」资源；旧协议名保留用于
  // 存量资源 / 工作台持久化 tab 的向后兼容着色与渲染。
  sql: 'dB',
  mysql: 'dB',
  postgresql: 'pg',
  redis: 'R',
  sqlite: 'S',
  s3: '☁',
  sip: '☎',
}

export const PROTOCOL_COLORS: Record<string, string> = {
  ssh: '#3FB950',
  sftp: '#8B5CF6',
  sql: '#58A6FF',
  mysql: '#58A6FF',
  postgresql: '#8B5CF6',
  redis: '#F85149',
  sqlite: '#D29922',
  s3: '#E8912D',
  sip: '#2DD4BF',
}

export const PROTOCOL_NAMES: Record<string, string> = {
  ssh: 'SSH',
  sftp: 'SFTP',
  sql: 'SQL',
  mysql: 'MySQL',
  postgresql: 'PostgreSQL',
  redis: 'Redis',
  sqlite: 'SQLite',
  s3: 'S3 / MinIO',
  sip: 'SIP Phone',
}

/** v0.70.7：SQL 资源的子类（dialect）元数据，用于按探测出的方言着色 / 显示图标。 */
export const SUBTYPE_META: Record<string, { icon: string; color: string; name: string }> = {
  mysql: { icon: 'dB', color: '#58A6FF', name: 'MySQL' },
  postgresql: { icon: 'pg', color: '#8B5CF6', name: 'PostgreSQL' },
  sqlite: { icon: 'S', color: '#D29922', name: 'SQLite' },
}

export type ProtocolType = keyof typeof PROTOCOL_ICONS
