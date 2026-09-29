// Static key table behind the shortcut panel, kept as pure data so both the
// component and its tests read one source. `title`/`desc` are i18n keys
// resolved at render time. `scope` disambiguates keys that do different
// things depending on focus (the three Ctrl+Shift+F meanings).
export type ShortcutScope = 'editorFocused' | 'editorBlurred' | 'terminalFocused'

export interface PanelShortcut {
  keys: string
  desc: string
  scope?: ShortcutScope
}

export interface PanelGroup {
  title: string
  shortcuts: PanelShortcut[]
}

export const SHORTCUT_PANEL_GROUPS: PanelGroup[] = [
  {
    title: 'shortcuts.groupWorkspace',
    shortcuts: [
      { keys: 'Ctrl+K', desc: 'shortcuts.globalSearch' },
      { keys: 'Ctrl+Shift+N', desc: 'shortcuts.newConnection' },
      { keys: 'Alt+T', desc: 'shortcuts.newTab' },
      { keys: 'Alt+W', desc: 'shortcuts.closeTab' },
      { keys: 'Ctrl+Shift+→', desc: 'shortcuts.nextTab' },
      { keys: 'Ctrl+Shift+←', desc: 'shortcuts.prevTab' },
      { keys: 'Alt+Shift+T', desc: 'shortcuts.reopenTab' },
      { keys: 'Alt+1~9', desc: 'shortcuts.jumpTab' },
      { keys: 'Ctrl+\\', desc: 'shortcuts.splitH' },
      { keys: 'Ctrl+Shift+\\', desc: 'shortcuts.splitV' },
      { keys: 'Ctrl+Alt+1', desc: 'shortcuts.layoutSingle' },
      { keys: 'Ctrl+Alt+2', desc: 'shortcuts.layoutLR' },
      { keys: 'Ctrl+Alt+3', desc: 'shortcuts.layoutTB' },
      { keys: 'Ctrl+Alt+4', desc: 'shortcuts.layoutGrid' },
      { keys: 'Ctrl+Alt+5', desc: 'shortcuts.layoutMain' },
      { keys: 'F11', desc: 'shortcuts.fullscreen' },
      { keys: 'F1', desc: 'shortcuts.toggleShortcuts' },
    ],
  },
  {
    title: 'shortcuts.groupSSH',
    shortcuts: [
      { keys: 'Ctrl+Shift+C', desc: 'shortcuts.copy' },
      { keys: 'Ctrl+Shift+V', desc: 'shortcuts.paste' },
      { keys: 'Ctrl+F', desc: 'shortcuts.findTerminal' },
      { keys: 'Ctrl+L', desc: 'shortcuts.clearScreen' },
      { keys: 'Ctrl+Shift+F', desc: 'terminal.openSftp', scope: 'terminalFocused' },
    ],
  },
  {
    title: 'shortcuts.groupSQL',
    shortcuts: [
      { keys: 'Ctrl+Enter', desc: 'shortcuts.execute' },
      { keys: 'Ctrl+Shift+F', desc: 'shortcuts.formatSQL', scope: 'editorFocused' },
      { keys: 'Ctrl+Shift+F', desc: 'shortcuts.globalSearch', scope: 'editorBlurred' },
      { keys: 'Ctrl+S', desc: 'shortcuts.saveQuery' },
      { keys: 'Ctrl+F', desc: 'shortcuts.find' },
      { keys: 'Ctrl+Shift+R', desc: 'shortcuts.findReplace' },
      { keys: 'Ctrl+Shift+Q', desc: 'shortcuts.globalQuery' },
      { keys: 'Ctrl+Shift+A', desc: 'shortcuts.aiAssistant' },
    ],
  },
  {
    title: 'shortcuts.groupFile',
    shortcuts: [
      { keys: 'F2', desc: 'shortcuts.renameFile' },
      { keys: 'F7', desc: 'shortcuts.newFolder' },
      { keys: 'F8 / Delete', desc: 'shortcuts.deleteFile' },
      { keys: 'Ctrl+R', desc: 'shortcuts.refreshFiles' },
    ],
  },
]
