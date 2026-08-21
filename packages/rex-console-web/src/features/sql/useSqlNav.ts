import { ref, type Ref } from 'vue'
import type { TableInfo } from '@/api/sql'
import * as sqlApi from '@/api/sql'

export interface DatabaseNode {
  name: string
  expanded: boolean
  tables: TableInfo[]
  loading: boolean
}

export function useSqlNav(sessionId: Ref<string | null>) {
  const databases = ref<DatabaseNode[]>([])
  const loading = ref(false)
  const searchQuery = ref('')

  async function loadDatabases() {
    if (!sessionId.value) return
    loading.value = true
    try {
      const dbs = await sqlApi.getDatabases(sessionId.value)
      databases.value = dbs.map((name) => ({
        name,
        expanded: false,
        tables: [],
        loading: false,
      }))
    } finally {
      loading.value = false
    }
  }

  async function toggleDatabase(db: DatabaseNode) {
    if (db.expanded) {
      db.expanded = false
      return
    }
    if (!sessionId.value) return
    db.loading = true
    try {
      db.tables = await sqlApi.getTables(sessionId.value, db.name)
      db.expanded = true
    } finally {
      db.loading = false
    }
  }

  function matchesSearch(db: DatabaseNode): boolean {
    if (!searchQuery.value) return true
    const q = searchQuery.value.toLowerCase()
    if (db.name.toLowerCase().includes(q)) return true
    return db.tables.some((t) => t.name.toLowerCase().includes(q))
  }

  function filteredDatabases(): DatabaseNode[] {
    return databases.value.filter(matchesSearch)
  }

  return {
    databases,
    loading,
    searchQuery,
    loadDatabases,
    toggleDatabase,
    filteredDatabases,
  }
}
