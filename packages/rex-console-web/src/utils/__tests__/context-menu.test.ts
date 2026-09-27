import { describe, it, expect } from 'vitest'
import {
  buildContextMenu,
  type MenuContext,
  type MenuItem,
  type NodeType,
} from '../context-menu'

const NODE_TYPES: NodeType[] = [
  'connection',
  'database',
  'table',
  'column',
  'file',
  'folder',
  'redis-key',
  'ssh-session',
  'tab',
]

function ctx(type: NodeType): MenuContext {
  return { type, data: {} }
}

function collectShortcuts(items: MenuItem[]): string[] {
  const out: string[] = []
  for (const item of items) {
    if (item.shortcut) out.push(item.shortcut)
    if (item.children) out.push(...collectShortcuts(item.children))
  }
  return out
}

describe('context-menu shortcut hints', () => {
  it('drops hints that have no real key binding', () => {
    // Mod+E (edit), Mod+C (copy), Mod+S (download), Mod+X (cut), Mod+W
    // (close) and Enter (open table) were display-only claims.
    expect(collectShortcuts(buildContextMenu('connection', ctx('connection')))).toEqual([])
    expect(collectShortcuts(buildContextMenu('table', ctx('table')))).toEqual([])
    expect(collectShortcuts(buildContextMenu('tab', ctx('tab')))).toEqual([])
    expect(collectShortcuts(buildContextMenu('database', ctx('database')))).toEqual([])
    expect(collectShortcuts(buildContextMenu('column', ctx('column')))).toEqual([])
    expect(collectShortcuts(buildContextMenu('redis-key', ctx('redis-key')))).toEqual([])
    expect(collectShortcuts(buildContextMenu('ssh-session', ctx('ssh-session')))).toEqual([])
  })

  it('keeps hints backed by real FilesPage bindings', () => {
    expect(collectShortcuts(buildContextMenu('file', ctx('file')))).toEqual(['F2', 'Del'])
    expect(collectShortcuts(buildContextMenu('folder', ctx('folder')))).toEqual(['F2', 'Del'])
  })

  it('emits no Mod-combination or Enter hints anywhere', () => {
    for (const type of NODE_TYPES) {
      const rendered = JSON.stringify(buildContextMenu(type, ctx(type)))
      expect(rendered, `${type} menu`).not.toMatch(/Mod\+/)
      expect(rendered, `${type} menu`).not.toMatch(/"Enter"/)
    }
  })
})
