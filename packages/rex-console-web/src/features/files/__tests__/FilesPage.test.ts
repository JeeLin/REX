import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import FilesPage from '../FilesPage.vue'

vi.mock('vue-i18n', () => ({ useI18n: () => ({ t: (k: string) => k }) }))

const { mockConnect, mockDisconnect, mockListFiles, mockGetCapability, mockPreviewSync, mockCreateSync } = vi.hoisted(() => ({
  mockConnect: vi.fn(),
  mockDisconnect: vi.fn(),
  mockListFiles: vi.fn(),
  mockGetCapability: vi.fn(),
  mockPreviewSync: vi.fn(),
  mockCreateSync: vi.fn(),
}))

const { mockStoreConnectWs, mockStoreDisconnectWs, mockStoreReconnectWs } = vi.hoisted(() => ({
  mockStoreConnectWs: vi.fn(),
  mockStoreDisconnectWs: vi.fn(),
  mockStoreReconnectWs: vi.fn(),
}))

vi.mock('@/api/files', () => ({
  connect: (...args: unknown[]) => mockConnect(...args),
  disconnect: (...args: unknown[]) => mockDisconnect(...args),
  listFiles: (...args: unknown[]) => mockListFiles(...args),
  getCapability: (...args: unknown[]) => mockGetCapability(...args),
  previewSync: (...args: unknown[]) => mockPreviewSync(...args),
  createSync: (...args: unknown[]) => mockCreateSync(...args),
}))

vi.mock('@/stores/transfer', () => ({
  useTransferStore: () => ({
    tasks: new Map(),
    pushBrowserTask: vi.fn(),
    updateBrowserTask: vi.fn(),
    move: vi.fn(),
    copy: vi.fn(),
    trackSync: vi.fn(),
    monitor: vi.fn(),
    cancel: vi.fn(),
    dismissCompleted: vi.fn(),
    connectWs: mockStoreConnectWs,
    reconnectWs: mockStoreReconnectWs,
    disconnectWs: mockStoreDisconnectWs,
  }),
}))

vi.mock('@/components/ui/Button.vue', () => ({
  default: {
    template: '<button><slot /></button>',
    props: ['variant', 'icon', 'size', 'disabled'],
  },
}))

vi.mock('../MobileFilesBar.vue', () => ({
  default: { template: '<div class="mobile-files-bar-stub" />', props: ['selectedCount'] },
}))

vi.mock('../FileEditorDialog.vue', () => ({
  default: { template: '<div class="file-editor-dialog-stub" />', props: ['visible', 'sessionId', 'filePath', 'protocol'] },
}))

beforeEach(() => {
  setActivePinia(createPinia())
  vi.clearAllMocks()
  mockConnect.mockResolvedValue('test-session-123')
  mockListFiles.mockResolvedValue([])
})

describe('FilesPage', () => {
  it('shows connect dialog when no resourceId is provided', () => {
    const wrapper = mount(FilesPage)
    expect(wrapper.find('.fp-overlay').exists()).toBe(true)
    expect(wrapper.text()).toContain('files.connectToServer')
  })

  it('auto-connects and loads panels when resourceId prop is provided', async () => {
    mount(FilesPage, { props: { resourceId: 'res-1', protocol: 'sftp' } })
    await flushPromises()

    expect(mockConnect).toHaveBeenCalledWith('res-1')
    expect(mockListFiles).toHaveBeenCalled()
  })

  it('emits update:status with connecting then online on successful connect', async () => {
    const wrapper = mount(FilesPage, { props: { resourceId: 'res-1' } })
    await flushPromises()

    const events = wrapper.emitted('update:status')!
    expect(events.some(e => e[0] === 'connecting')).toBe(true)
    expect(events.some(e => e[0] === 'online')).toBe(true)
  })

  it('renders dual panels after successful connection', async () => {
    const wrapper = mount(FilesPage, { props: { resourceId: 'res-1' } })
    await flushPromises()

    const panels = wrapper.findAll('.fp-panel')
    expect(panels.length).toBe(2)
  })

  it('shows empty state when listing returns no entries', async () => {
    mockListFiles.mockResolvedValue([])
    const wrapper = mount(FilesPage, { props: { resourceId: 'res-1' } })
    await flushPromises()

    expect(wrapper.find('.pe').exists()).toBe(true)
    expect(wrapper.text()).toContain('files.empty')
  })
})
