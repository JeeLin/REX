//! Shared types for the files feature domain (v0.91.0 T6).
//! Re-exports canonical types and defines local-only shapes used across the
//! file pages, composables, and child components.

import type { FileEntry, FileCapabilities, FileCapability } from '@/api/files'
import type { TransferItem, TransferKind, TransferDirection, TransferStatus } from '@/stores/transfer'

export type { FileEntry, FileCapabilities, FileCapability }
export type { TransferItem, TransferKind, TransferDirection, TransferStatus }

export type Side = 'left' | 'right'

/** A panel's browsing/selection state. */
export interface FilesPanel {
  path: string
  entries: FileEntry[]
  loading: boolean
  selected: Set<string>
  active: boolean
}

/** Notification tone accepted by the toast component. */
export type ToastTone = 'success' | 'error' | 'info' | 'warning'

export interface UseFilesOptions {
  resourceId: string | undefined
  protocol: string
  tabId: string | undefined
  onStatus: (status: string) => void
  notify: (message: string, tone: ToastTone) => void
}
