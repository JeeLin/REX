import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import WorkspaceSql from '../panels/WorkspaceSql.vue'

// Create i18n instance
const i18n = createI18n({
  legacy: false,
  locale: 'zh',
  messages: {
    zh: {
      common: { cancel: '取消', save: '保存' },
      sql: {
        execute: '执行',
        format: '格式化',
        saveQuery: '保存查询',
        fileName: '文件名',
        fileNamePlaceholder: '输入查询名称',
        toast: { saveSuccess: '保存成功', saveFailed: '保存失败' },
        sidebar: { renamePrompt: '输入新名称:' },
      },
    },
  },
})

// Mock dependencies
vi.mock('@/api/sql', () => ({
  listDatabases: vi.fn().mockResolvedValue([]),
  executeSql: vi.fn().mockResolvedValue({ columns: [], rows: [], affected_rows: 0, elapsed_ms: 0 }),
  saveQuery: vi.fn().mockResolvedValue({ id: '1', name: 'test', database: 'test', created_at: '', updated_at: '' }),
}))

vi.mock('@/features/sql/useSqlTabActions', () => ({
  useSqlTabActions: vi.fn(() => ({
    tabs: { value: [{ id: '1', title: 'Query 1', sql: 'SELECT 1', result: null, queryId: null }] },
    activeTabId: { value: '1' },
    executing: { value: false },
    tabList: { value: [{ id: '1', title: 'Query 1', queryId: null }] },
    activeTab: { value: { id: '1', title: 'Query 1', sql: 'SELECT 1', result: null, queryId: null } },
    addTab: vi.fn(),
    closeTab: vi.fn(),
    closeOthers: vi.fn(),
    renameTab: vi.fn(),
    getTabSql: vi.fn().mockReturnValue('SELECT 1'),
    clearEditor: vi.fn(),
    execute: vi.fn(),
    handleSort: vi.fn(),
    handleGenerateSql: vi.fn(),
    markSaved: vi.fn(),
  })),
}))

vi.mock('@/composables/useToast', () => ({
  useToast: vi.fn(() => ({
    success: vi.fn(),
    error: vi.fn(),
  })),
}))

describe('WorkspaceSql', () => {
  const defaultProps = {
    resourceId: 'res-1',
    resourceName: 'Test DB',
    protocol: 'mysql',
  }

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders toolbar with execute, format, save, and clear buttons', () => {
    const wrapper = mount(WorkspaceSql, {
      props: defaultProps,
      global: { plugins: [i18n] },
    })
    const toolbar = wrapper.find('.ws-sql-toolbar')
    expect(toolbar.exists()).toBe(true)
    expect(toolbar.text()).toContain('执行')
    expect(toolbar.text()).toContain('格式化')
    expect(toolbar.text()).toContain('保存')
    expect(toolbar.text()).toContain('清空')
  })

  it('renders status bar with resource name', () => {
    const wrapper = mount(WorkspaceSql, {
      props: defaultProps,
      global: { plugins: [i18n] },
    })
    const statusbar = wrapper.find('.ws-sql-statusbar')
    expect(statusbar.exists()).toBe(true)
    expect(statusbar.text()).toContain('Test DB')
  })

  it('shows save modal when save button is clicked', async () => {
    const wrapper = mount(WorkspaceSql, {
      props: defaultProps,
      global: { plugins: [i18n] },
    })
    const saveButton = wrapper.findAll('button').find(b => b.text().includes('保存'))
    expect(saveButton).toBeDefined()
    await saveButton!.trigger('click')
    expect(wrapper.find('.ws-sql-modal-overlay').exists()).toBe(true)
  })

  it('hides save modal when cancel is clicked', async () => {
    const wrapper = mount(WorkspaceSql, {
      props: defaultProps,
      global: { plugins: [i18n] },
    })
    const saveButton = wrapper.findAll('button').find(b => b.text().includes('保存'))
    await saveButton!.trigger('click')
    expect(wrapper.find('.ws-sql-modal-overlay').exists()).toBe(true)
    const cancelButton = wrapper.findAll('button').find(b => b.text().includes('取消'))
    await cancelButton!.trigger('click')
    expect(wrapper.find('.ws-sql-modal-overlay').exists()).toBe(false)
  })
})
