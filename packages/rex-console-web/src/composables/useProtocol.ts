import { useRouter } from 'vue-router'
import { useRecent } from './useRecent'
import { useTabs } from '@/features/workspace/useTabs'

export type Protocol = 'ssh' | 'sftp' | 'mysql' | 'postgresql' | 'redis' | 'sqlite' | 's3'

/** Protocol → icon/label mapping */
const PROTOCOL_ICONS: Record<string, { icon: string; color: string }> = {
  ssh: { icon: '$', color: '#22d3ee' },
  sftp: { icon: '📁', color: '#a78bfa' },
  mysql: { icon: 'dB', color: '#f59e0b' },
  postgresql: { icon: 'pg', color: '#60a5fa' },
  redis: { icon: 'r', color: '#ef4444' },
  sqlite: { icon: 'db', color: '#10b981' },
  s3: { icon: '☁', color: '#f97316' },
}

export function getProtocolIcon(protocol: string) {
  return PROTOCOL_ICONS[protocol] ?? { icon: '?', color: '#888' }
}

/** Connect resource → workspace tab */
export function useProtocol() {
  const router = useRouter()
  const { addToRecent } = useRecent()
  const { addTab } = useTabs()

  function connectToResource(
    resource: { id: string; protocol: string; name: string },
    envName: string,
  ) {
    addTab(resource.name, resource.protocol as Protocol, resource.id)
    router.push('/workspace')
    addToRecent({ resourceId: resource.id, name: resource.name, protocol: resource.protocol, envName })
  }

  return { connectToResource }
}
