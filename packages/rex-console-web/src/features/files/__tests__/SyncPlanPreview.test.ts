import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import SyncPlanPreview from '../SyncPlanPreview.vue'
import { previewSync, type SyncPlan, type SyncRequestBody } from '@/api/files'

// 组件通过 vue-i18n 的 t() 取文案，测试按点分路径解析真实 locale 文件后断言。
vi.mock('vue-i18n', async () => {
  const locale = (await import('@/i18n/locales/zh.json')).default as unknown as Record<string, unknown>
  return {
    useI18n: () => ({
      t: (key: string, fallback?: string) => {
        let node: unknown = locale
        for (const part of key.split('.')) {
          if (node === null || typeof node !== 'object') return fallback ?? key
          node = (node as Record<string, unknown>)[part]
        }
        return typeof node === 'string' ? node : (fallback ?? key)
      },
    }),
  }
})

const plan: SyncPlan = {
  actions: [
    { rel_path: 'app.rs', action: 'copy', dir: 'to_target', size: 12, source_mtime: 1767323045, target_mtime: null },
    { rel_path: 'docs/old.md', action: 'copy', dir: 'to_source', size: 2048, source_mtime: null, target_mtime: 1767323000 },
    { rel_path: 'vendor/lib.js', action: 'delete', dir: 'to_target', size: 4096 },
    { rel_path: 'conflict.txt', action: 'conflict', dir: 'to_target', size: 3, source_mtime: null, target_mtime: null },
  ],
  summary: { copies: 2, deletes: 1, conflicts: 1, total_bytes: 2063 },
}

describe('SyncPlanPreview', () => {
  it('renders one row per action with direction-aware labels', () => {
    const w = mount(SyncPlanPreview, { props: { plan } })
    const rows = w.findAll('.sp-table tbody tr')
    expect(rows).toHaveLength(4)

    // 复制方向以 ↑/↓ 区分（to_target = 源→目标），删除 / 冲突不带方向。
    expect(rows[0]!.find('.sp-path').text()).toBe('app.rs')
    expect(rows[0]!.find('.sp-act').text()).toBe('复制 ↑')
    expect(rows[0]!.find('.sp-act').classes()).toContain('sp-act--up')
    expect(rows[1]!.find('.sp-act').text()).toBe('复制 ↓')
    expect(rows[1]!.find('.sp-act').classes()).toContain('sp-act--down')
    expect(rows[2]!.find('.sp-act').text()).toBe('删除')
    expect(rows[2]!.find('.sp-act').classes()).toContain('sp-act--delete')
    expect(rows[3]!.find('.sp-act').text()).toBe('冲突')
    expect(rows[3]!.find('.sp-act').classes()).toContain('sp-act--conflict')
  })

  it('shows summary badges and total bytes', () => {
    const w = mount(SyncPlanPreview, { props: { plan } })
    const badges = w.findAll('.sp-badge').map(b => b.text())
    expect(badges).toEqual(['复制 2', '删除 1', '冲突 1'])
    expect(w.find('.sp-total').text()).toBe('2.0 KB')
  })

  it('renders both mtime columns and marks unreadable times as unknown', () => {
    const w = mount(SyncPlanPreview, { props: { plan } })
    const headers = w.findAll('.sp-table th').map(th => th.text())
    expect(headers).toEqual(['路径', '操作', '大小', '源修改时间', '目标修改时间'])

    const firstRowCells = w.findAll('.sp-table tbody tr')[0]!.findAll('td')
    expect(firstRowCells[2]!.text()).toBe('12 B')
    expect(firstRowCells[3]!.text()).not.toBe('未知')
    // 对侧时间缺失（S3/SFTP 未返回可解析时间）如实显示未知。
    expect(firstRowCells[4]!.text()).toBe('未知')
  })

  it('states the deletion / conflict / mtime caveats instead of over-promising', () => {
    const w = mount(SyncPlanPreview, { props: { plan } })
    const notes = w.findAll('.sp-notes li').map(li => li.text())
    // 空目录不被同步、孤儿删除只删文件、时间不可读时退化为仅比大小、冲突取较新者。
    expect(notes).toHaveLength(4)
    expect(notes[0]).toContain('空目录不会被同步')
    expect(notes[1]).toContain('只删文件')
    expect(notes[2]).toContain('仅比大小')
    expect(notes[3]).toContain('较新者为准')
  })

  it('shows the up-to-date state for an empty plan', () => {
    const w = mount(SyncPlanPreview, {
      props: { plan: { actions: [], summary: { copies: 0, deletes: 0, conflicts: 0, total_bytes: 0 } } },
    })
    expect(w.find('.sp-state--ok').text()).toBe('已是最新')
    expect(w.find('.sp-table').exists()).toBe(false)
    expect(w.findAll('.sp-badge').map(b => b.text())).toEqual(['复制 0', '删除 0', '冲突 0'])
  })

  it('shows loading and error states instead of an empty table', () => {
    const loading = mount(SyncPlanPreview, { props: { plan: null, loading: true } })
    expect(loading.find('.sp-state').text()).toBe('正在对比目录...')
    expect(loading.find('.sp-table').exists()).toBe(false)

    const failed = mount(SyncPlanPreview, { props: { plan: null, error: 'scan failed' } })
    expect(failed.find('.sp-state--err').text()).toBe('scan failed')
    expect(failed.find('.sp-table').exists()).toBe(false)
  })

  it('renders nothing to execute — the component only displays the given plan', () => {
    const w = mount(SyncPlanPreview, { props: { plan } })
    expect(w.findAll('button')).toHaveLength(0)
  })
})

describe('previewSync', () => {
  beforeEach(() => {
    localStorage.setItem('rex-token', 'tok')
  })
  afterEach(() => {
    vi.unstubAllGlobals()
    vi.restoreAllMocks()
  })

  it('posts the sync request body to the dry-run endpoint', async () => {
    const body: SyncRequestBody = {
      source: { resource_id: 'res-src', path: '/src/' },
      target: { resource_id: 'res-dst', path: '/dst/' },
      options: { direction: 'upload', compare: 'modified_time', include: [], exclude: ['*.log'], delete_orphans: true },
      conflict: 'overwrite',
    }
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => plan,
    })
    vi.stubGlobal('fetch', fetchMock)

    const got = await previewSync(body)
    expect(got).toEqual(plan)

    const [url, init] = fetchMock.mock.calls[0]! as [string, RequestInit]
    expect(url).toBe('/api/files/sync/preview')
    expect(init.method).toBe('POST')
    expect(JSON.parse(init.body as string)).toEqual(body)
    expect((init.headers as Record<string, string>).Authorization).toBe('Bearer tok')
  })

  it('raises the backend error code on failure', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: false,
      status: 400,
      statusText: 'Bad Request',
      json: async () => ({ error: { code: 'SYNC_PATH_REQUIRED', message: 'source and target path are required' } }),
    }))

    const body: SyncRequestBody = {
      source: { resource_id: 'a', path: '' },
      target: { resource_id: 'b', path: '/dst/' },
      options: { direction: 'upload', compare: 'modified_time', include: [], exclude: [], delete_orphans: false },
    }
    await expect(previewSync(body)).rejects.toMatchObject({ code: 'SYNC_PATH_REQUIRED' })
  })
})