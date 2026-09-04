import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import FileEditorDialog from '../FileEditorDialog.vue'

vi.mock('vue-i18n', () => ({ useI18n: () => ({ t: (k: string) => k }) }))

const mockReadForEdit = vi.fn()
const mockSaveFromEdit = vi.fn()

vi.mock('@/api/files', () => ({
  readForEdit: (...args: unknown[]) => mockReadForEdit(...args),
  saveFromEdit: (...args: unknown[]) => mockSaveFromEdit(...args),
}))

vi.mock('@/components/ui/Button.vue', () => ({
  default: {
    // No @click emit — parent @click falls through as native listener
    template: '<button :disabled="disabled"><slot /></button>',
    props: ['variant', 'icon', 'size', 'disabled'],
  },
}))

beforeEach(() => {
  vi.clearAllMocks()
  mockReadForEdit.mockResolvedValue({
    filename: 'test.ts',
    content: btoa('hello world'),
    size: 11,
  })
})

// Base props with visible=false so the watch can be triggered
const baseProps = {
  visible: false,
  sessionId: 'sess-1',
  filePath: '/home/user/test.ts',
  protocol: 'sftp' as const,
}

// Stub Teleport so its children render inline in the wrapper DOM
const mountOpts = { global: { stubs: { Teleport: true } } }

describe('FileEditorDialog', () => {
  it('loads file content when becoming visible', async () => {
    const wrapper = mount(FileEditorDialog, { ...mountOpts, props: baseProps })
    await wrapper.setProps({ visible: true })
    await flushPromises()

    expect(mockReadForEdit).toHaveBeenCalledWith('sess-1', '/home/user/test.ts')
    expect(wrapper.text()).toContain('test.ts')
  })

  it('emits close when close button is clicked', async () => {
    const wrapper = mount(FileEditorDialog, { ...mountOpts, props: { ...baseProps, visible: true } })
    await flushPromises()

    await wrapper.find('.editor-actions button:last-child').trigger('click')
    expect(wrapper.emitted('close')).toHaveLength(1)
  })

  it('emits saved after successful save', async () => {
    const wrapper = mount(FileEditorDialog, { ...mountOpts, props: baseProps })
    await wrapper.setProps({ visible: true })
    await flushPromises()

    // The save button is the first button in .editor-actions
    const saveBtn = wrapper.find('.editor-actions button:first-child')
    await saveBtn.trigger('click')
    await flushPromises()

    expect(mockSaveFromEdit).toHaveBeenCalled()
    expect(wrapper.emitted('saved')).toHaveLength(1)
  })

  it('does not render dialog content when visible is false', () => {
    const wrapper = mount(FileEditorDialog, {
      ...mountOpts,
      props: baseProps,
    })

    expect(wrapper.find('.editor-dialog').exists()).toBe(false)
  })

  it('shows loading state while fetching file content', async () => {
    // Make readForEdit never resolve to keep loading state
    mockReadForEdit.mockReturnValue(new Promise(() => {}))
    const wrapper = mount(FileEditorDialog, { ...mountOpts, props: baseProps })
    await wrapper.setProps({ visible: true })
    await flushPromises()

    expect(wrapper.find('.editor-loading').exists()).toBe(true)
    expect(wrapper.text()).toContain('files.loadingFile')
  })
})
