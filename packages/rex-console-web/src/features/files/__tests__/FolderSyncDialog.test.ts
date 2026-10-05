import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import FolderSyncDialog from '../FolderSyncDialog.vue'
import type { SyncRequestBody, TransferEndpoint } from '@/api/files'

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

const { mockPreviewSync, mockCreateSync } = vi.hoisted(() => ({
  mockPreviewSync: vi.fn(),
  mockCreateSync: vi.fn(),
}))

vi.mock('@/api/files', () => ({
  previewSync: (...args: unknown[]) => mockPreviewSync(...args),
  createSync: (...args: unknown[]) => mockCreateSync(...args),
}))

// Button stub：不 emit click，父组件的 @click 作为原生监听器兜底。
vi.mock('@/components/ui/Button.vue', () => ({
  default: {
    template: '<button :disabled="disabled" :class="variant"><slot /></button>',
    props: ['variant', 'size', 'disabled', 'loading', 'icon'],
  },
}))

// Stub Teleport so its children render inline in the wrapper DOM
const mountOpts = { global: { stubs: { Teleport: true } } }

const SOURCE: TransferEndpoint = { resource_id: 'res-1', path: '/left/dir/' }
const TARGET: TransferEndpoint = { resource_id: 'res-1', path: '/right/dir/' }

const emptyPlan = { actions: [], summary: { copies: 0, deletes: 0, conflicts: 0, total_bytes: 0 } }

function mountDialog(overrides: Record<string, unknown> = {}) {
  return mount(FolderSyncDialog, {
    ...mountOpts,
    props: { open: true, source: SOURCE, target: TARGET, ...overrides } as never,
  })
}

async function chooseRadio(w: ReturnType<typeof mountDialog>, value: string) {
  const input = w.find(`input[type="radio"][value="${value}"]`)
  ;(input.element as HTMLInputElement).checked = true
  await input.trigger('change')
}

beforeEach(() => {
  vi.clearAllMocks()
  mockPreviewSync.mockResolvedValue(emptyPlan)
  mockCreateSync.mockResolvedValue({ id: 'sync-1', status: 'pending' })
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('FolderSyncDialog', () => {
  it('renders the five sync levers (direction / compare / masks / orphans / preview)', () => {
    const w = mountDialog()

    const directions = w.findAll('input[type="radio"][name], .fsd-radios input[type="radio"]')
    expect(directions).toHaveLength(5)
    expect(directions.slice(0, 3).map((i) => (i.element as HTMLInputElement).value)).toEqual([
      'upload',
      'download',
      'bidirectional',
    ])
    expect(directions.slice(3).map((i) => (i.element as HTMLInputElement).value)).toEqual([
      'size',
      'modified_time',
    ])
    expect(w.find('#fsd-include').exists()).toBe(true)
    expect(w.find('#fsd-exclude').exists()).toBe(true)
    expect(w.find('input[type="checkbox"]').exists()).toBe(true)
    expect(w.findAll('.fsd-actions button').map((b) => b.text())).toEqual(['取消', '预览', '开始同步'])
  })

  it('shows both endpoints so the user knows which roots are compared', () => {
    const w = mountDialog()
    const paths = w.findAll('.fsd-endpoint-path').map((p) => p.text())
    expect(paths).toEqual(['/left/dir/', '/right/dir/'])
  })

  it('splits multi-line masks, trims them and drops blank lines', async () => {
    const w = mountDialog()
    await w.find('#fsd-include').setValue(' *.rs\n\nsrc/**\n')
    await w.find('#fsd-exclude').setValue('*.log\nnode_modules/\n\n')

    await w.find('.fsd-actions button:nth-child(2)').trigger('click')
    await flushPromises()

    const body = mockPreviewSync.mock.calls[0]![0] as SyncRequestBody
    expect(body.options.include).toEqual(['*.rs', 'src/**'])
    expect(body.options.exclude).toEqual(['*.log', 'node_modules/'])
  })

  it('preview and start send the very same options — what you see is what runs', async () => {
    const w = mountDialog()
    await chooseRadio(w, 'download')
    await chooseRadio(w, 'size')
    await w.find('#fsd-include').setValue('docs/**')
    await w.find('#fsd-exclude').setValue('*.tmp')
    const checkbox = w.find('input[type="checkbox"]')
    await checkbox.setValue(true)

    await w.find('.fsd-actions button:nth-child(2)').trigger('click')
    await flushPromises()
    await w.find('.fsd-actions button:nth-child(3)').trigger('click')
    await flushPromises()

    const previewed = mockPreviewSync.mock.calls[0]![0] as SyncRequestBody
    const executed = mockCreateSync.mock.calls[0]![0] as SyncRequestBody
    expect(previewed.options).toEqual({
      direction: 'download',
      compare: 'size',
      include: ['docs/**'],
      exclude: ['*.tmp'],
      delete_orphans: true,
    })
    expect(executed).toEqual(previewed)
  })

  it('marks the preview stale when options change after a preview', async () => {
    const w = mountDialog()
    await w.find('.fsd-actions button:nth-child(2)').trigger('click')
    await flushPromises()
    expect(mockPreviewSync).toHaveBeenCalledTimes(1)

    await w.find('#fsd-exclude').setValue('*.log')
    await flushPromises()

    // 旧计划不再代表当前配置：清空 + 提示重新预览。
    expect(w.find('.fsd-hint--stale').text()).toBe('选项已更改，预览已失效——请重新预览。')
    expect(w.find('.sp-table').exists()).toBe(false)
  })

  it('disables the orphan switch on bidirectional and never sends delete_orphans', async () => {
    const w = mountDialog()
    const checkbox = w.find('input[type="checkbox"]')
    await checkbox.setValue(true)

    await chooseRadio(w, 'bidirectional')
    await flushPromises()

    expect((w.find('input[type="checkbox"]').element as HTMLInputElement).disabled).toBe(true)
    expect(w.findAll('.fsd-hint').map((h) => h.text())).toContain(
      '双向同步两侧都保留，不会删除任何文件。',
    )

    await w.find('.fsd-actions button:nth-child(3)').trigger('click')
    await flushPromises()
    const body = mockCreateSync.mock.calls[0]![0] as SyncRequestBody
    expect(body.options.direction).toBe('bidirectional')
    expect(body.options.delete_orphans).toBe(false)
  })

  it('disables both actions while the endpoints are missing', () => {
    const w = mountDialog({ target: null })
    const buttons = w.findAll('.fsd-actions button')
    expect((buttons[1]!.element as HTMLButtonElement).disabled).toBe(true)
    expect((buttons[2]!.element as HTMLButtonElement).disabled).toBe(true)
  })

  it('shows the preview error and reports it to the caller', async () => {
    mockPreviewSync.mockRejectedValue(new Error('scan failed'))
    const w = mountDialog()

    await w.find('.fsd-actions button:nth-child(2)').trigger('click')
    await flushPromises()

    expect(w.find('.sp-state--err').text()).toBe('scan failed')
    expect(w.emitted('error')![0]![0]).toBe('预览失败: scan failed')
  })

  it('shows the create error inline and keeps the dialog open', async () => {
    mockCreateSync.mockRejectedValue(new Error('SYNC_PATH_REQUIRED: source and target path are required'))
    const w = mountDialog()

    await w.find('.fsd-actions button:nth-child(3)').trigger('click')
    await flushPromises()

    expect(w.find('.fsd-error').text()).toBe(
      'SYNC_PATH_REQUIRED: source and target path are required',
    )
    expect(w.emitted('error')![0]![0]).toContain('同步失败')
    expect(w.emitted('close')).toBeUndefined()
  })

  it('emits created + close after a successful sync task creation', async () => {
    const w = mountDialog()
    await w.find('.fsd-actions button:nth-child(3)').trigger('click')
    await flushPromises()

    expect(w.emitted('created')![0]![0]).toBe('sync-1')
    expect(w.emitted('close')).toHaveLength(1)
  })

  it('emits close on cancel without touching the API', async () => {
    const w = mountDialog()
    await w.find('.fsd-actions button:nth-child(1)').trigger('click')
    expect(w.emitted('close')).toHaveLength(1)
    expect(mockPreviewSync).not.toHaveBeenCalled()
    expect(mockCreateSync).not.toHaveBeenCalled()
  })

  it('resets the option state and the plan each time the dialog is reopened', async () => {
    const w = mountDialog()
    await chooseRadio(w, 'download')
    await w.find('#fsd-exclude').setValue('*.log')
    await w.find('.fsd-actions button:nth-child(2)').trigger('click')
    await flushPromises()

    await w.setProps({ open: false })
    await w.setProps({ open: true })
    await flushPromises()

    expect((w.find('input[type="radio"][value="upload"]').element as HTMLInputElement).checked).toBe(true)
    expect((w.find('input[type="radio"][value="size"]').element as HTMLInputElement).checked).toBe(false)
    expect((w.find('#fsd-exclude').element as HTMLTextAreaElement).value).toBe('')
    expect(w.find('.fsd-plan').exists()).toBe(false)
    expect(w.find('.fsd-hint--stale').exists()).toBe(false)
  })
})
