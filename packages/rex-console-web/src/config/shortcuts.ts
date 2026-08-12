export interface ShortcutEntry {
  id: string
  keys: string
  description: string
  category: 'workspace' | 'tab' | 'split' | 'nav'
}

export const SHORTCUTS: ShortcutEntry[] = [
  // Workspace
  { id: 'cmd-palette', keys: 'Ctrl+K', description: 'Command palette', category: 'workspace' },
  { id: 'toggle-sftp', keys: 'Ctrl+J', description: 'Toggle SFTP drawer', category: 'workspace' },
  { id: 'broadcast-input', keys: 'Ctrl+Shift+B', description: 'Toggle broadcast input', category: 'workspace' },

  // Tab
  { id: 'new-ssh', keys: 'Ctrl+T', description: 'New SSH connection', category: 'tab' },
  { id: 'new-tab', keys: 'Ctrl+N', description: 'New connection (quick connect)', category: 'tab' },
  { id: 'close-tab', keys: 'Ctrl+W', description: 'Close active tab', category: 'tab' },
  { id: 'tab-1', keys: 'Alt+1', description: 'Switch to tab 1', category: 'tab' },
  { id: 'tab-2', keys: 'Alt+2', description: 'Switch to tab 2', category: 'tab' },
  { id: 'tab-3', keys: 'Alt+3', description: 'Switch to tab 3', category: 'tab' },
  { id: 'tab-4', keys: 'Alt+4', description: 'Switch to tab 4', category: 'tab' },
  { id: 'tab-5', keys: 'Alt+5', description: 'Switch to tab 5', category: 'tab' },
  { id: 'tab-6', keys: 'Alt+6', description: 'Switch to tab 6', category: 'tab' },
  { id: 'tab-7', keys: 'Alt+7', description: 'Switch to tab 7', category: 'tab' },
  { id: 'tab-8', keys: 'Alt+8', description: 'Switch to tab 8', category: 'tab' },
  { id: 'tab-9', keys: 'Alt+9', description: 'Switch to tab 9', category: 'tab' },
  { id: 'next-tab', keys: 'Ctrl+Tab', description: 'Next tab', category: 'tab' },

  // Split
  { id: 'split-h', keys: 'Ctrl+\\', description: 'Split horizontal', category: 'split' },
  { id: 'split-v', keys: 'Ctrl+Shift+\\', description: 'Split vertical', category: 'split' },
  { id: 'layout-single', keys: 'Alt+1', description: 'Layout: single panel', category: 'split' },
  { id: 'layout-lr', keys: 'Alt+2', description: 'Layout: left-right', category: 'split' },
  { id: 'layout-tb', keys: 'Alt+3', description: 'Layout: top-bottom', category: 'split' },
  { id: 'layout-grid', keys: 'Alt+4', description: 'Layout: grid four', category: 'split' },
  { id: 'layout-main', keys: 'Alt+5', description: 'Layout: main+side', category: 'split' },

  // Navigation
  { id: 'nav-workspace', keys: 'Alt+W', description: 'Go to workspace', category: 'nav' },
  { id: 'nav-dashboard', keys: 'Alt+D', description: 'Go to dashboard', category: 'nav' },
]
