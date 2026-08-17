/** 协议图标、颜色、名称映射 */
export const PROTOCOL_ICONS: Record<string, string> = {
  ssh: '$',
  sftp: '📁',
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
  mysql: 'MySQL',
  postgresql: 'PostgreSQL',
  redis: 'Redis',
  sqlite: 'SQLite',
  s3: 'S3 / MinIO',
  sip: 'SIP Phone',
}

export type ProtocolType = keyof typeof PROTOCOL_ICONS
