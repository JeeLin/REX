import { useRouter } from 'vue-router'
import { useRecent } from './useRecent'

export type Protocol = 'ssh' | 'sftp' | 'mysql' | 'postgresql' | 'redis' | 'docker' | 'sqlite' | 's3'

/** Protocol → icon/label mapping */
const PROTOCOL_ICONS: Record<string, { icon: string; color: string }> = {
  ssh: { icon: '$', color: '#22d3ee' },
  sftp: { icon: '📁', color: '#a78bfa' },
  mysql: { icon: 'dB', color: '#f59e0b' },
  postgresql: { icon: 'pg', color: '#60a5fa' },
  redis: { icon: 'r', color: '#ef4444' },
  docker: { icon: '🐳', color: '#3b82f6' },
  sqlite: { icon: 'db', color: '#10b981' },
  s3: { icon: '☁', color: '#f97316' },
}

export function getProtocolIcon(protocol: string) {
  return PROTOCOL_ICONS[protocol] ?? { icon: '?', color: '#888' }
}

/** Protocol routing: resource click → navigate to correct page */
export function useProtocol() {
  const router = useRouter()
  const { addToRecent } = useRecent()

  function connectToResource(
    resource: { id: string; protocol: string; name: string },
    envName: string,
  ) {
    switch (resource.protocol) {
      case 'ssh':
        router.push(`/terminal/${resource.id}`)
        break
      case 'sftp':
        router.push(`/files/${resource.id}`)
        break
      case 'mysql':
      case 'postgresql':
      case 'redis':
      case 'sqlite':
        router.push(`/sql/${resource.id}`)
        break
      default:
        return // docker, s3 — not yet supported, skip recent recording
    }
    addToRecent({ resourceId: resource.id, name: resource.name, protocol: resource.protocol, envName })
  }

  return { connectToResource }
}
