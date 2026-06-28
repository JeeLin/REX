import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { listFiles, mkdirFile, touchFile, deleteFile as apiDeleteFile, renameFile } from '@/api/files'
import type { FileEntry } from '@/api/files'
import { getErrorMessage } from '@/utils/error'

export function useFileManager(resourceId: string) {
  const { t } = useI18n()
  const currentPath = ref('/')
  const entries = ref<FileEntry[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function loadFiles(path?: string) {
    if (path !== undefined) {
      currentPath.value = path
    }
    loading.value = true
    error.value = null
    try {
      const data = await listFiles(resourceId, currentPath.value)
      currentPath.value = data.path
      entries.value = data.entries.sort((a, b) => {
        if (a.file_type !== b.file_type) {
          return a.file_type === 'directory' ? -1 : 1
        }
        return a.name.localeCompare(b.name)
      })
    } catch (e: unknown) {
      error.value = getErrorMessage(e, t('files.loadError'))
    } finally {
      loading.value = false
    }
  }

  function navigateTo(path: string) {
    loadFiles(path)
  }

  function enterDirectory(name: string) {
    const base = currentPath.value === '/' ? '/' : currentPath.value + '/'
    loadFiles(base + name)
  }

  function goUp() {
    if (currentPath.value === '/') return
    const parent = currentPath.value.replace(/\/[^/]+\/?$/, '') || '/'
    loadFiles(parent)
  }

  async function createDir(name: string) {
    const base = currentPath.value === '/' ? '/' : currentPath.value + '/'
    await mkdirFile(resourceId, base + name)
    await loadFiles()
  }

  async function createFile(name: string) {
    const base = currentPath.value === '/' ? '/' : currentPath.value + '/'
    await touchFile(resourceId, base + name)
    await loadFiles()
  }

  async function deleteEntries(paths: string[]) {
    for (const p of paths) {
      await apiDeleteFile(resourceId, p)
    }
    await loadFiles()
  }

  async function renameEntry(oldPath: string, newName: string) {
    const dir = oldPath.replace(/\/[^/]+$/, '') || '/'
    const base = dir === '/' ? '/' : dir + '/'
    await renameFile(resourceId, oldPath, base + newName)
    await loadFiles()
  }

  return {
    currentPath,
    entries,
    loading,
    error,
    loadFiles,
    navigateTo,
    enterDirectory,
    goUp,
    createDir,
    createFile,
    deleteEntries,
    renameEntry,
  }
}
