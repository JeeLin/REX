import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount, flushPromises, type VueWrapper } from '@vue/test-utils'
import FilesPage from '../FilesPage.vue'
import { buildPaneCtx, mountInPane } from '@/features/workspace/__tests__/paneCtx'

vi.mock('vue-i18n', () => ({ useI18n: () => ({ t: (k: string) => k }) }))

const mockConnect = vi.fn()
const mockListFiles = vi.fn()
const mockDisconnect = vi.fn()
const mockMkdir = vi.fn()
const mockDeleteFile = vi.fn()

vi.mock('@/api/files', () => ({
  connect: (...args: unknown[]) => mockConnect(...args),
  disconnect: (...args: unknown[]) => mockDisconnect(...args),
  listFiles: (...args: unknown[]) => mockListFiles(...args),
  mkdir: (...args: unknown[]) => mockMkdir(...args),
  deleteFile: (...args: unknown[]) => mockDeleteFile(...args),
}))

vi.mock('@/components/ui/Button.vue', () => ({
  default: {
    template: '<button><slot /></button>',
    props: ['variant', 'icon', 'size', 'disabled'],
  },
}))

vi.mock('../FolderSyncDialog.vue', () => ({
  default: { template: '<div class="folder-sync-dialog-stub" />', props: ['visible', 'sourcePath', 'targetPath'] },
}))

vi.mock('../MobileFilesBar.vue', () => ({
  default: { template: '<div class="mobile-files-bar-stub" />', props: ['selectedCount'] },
}))

vi.mock('../FileEditorDialog.vue', () => ({
  default: { template: '<div class="file-editor-dialog-stub" />', props: ['visible', 'sessionId', 'filePath', 'protocol'] },
}))

let wrapper: VueWrapper | undefined
const extraWrappers: VueWrapper[] = []
let promptMock: ReturnType<typeof vi.fn>

// Mounts a FilesPage instance inside a pane context and registers it for cleanup.
function mountFilesInPane(
  resourceId: string,
  tabId: string,
  ctx: ReturnType<typeof buildPaneCtx>,
): VueWrapper {
  const w = mountInPane(FilesPage, ctx, { resourceId, protocol: 'sftp', tabId })
  extraWrappers.push(w)
  return w
}

function press(target: Element | Document, key: string, init: KeyboardEventInit = {}): KeyboardEvent {
  const event = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init })
  target.dispatchEvent(event)
  return event
}

async function mountConnected() {
  mockListFiles.mockResolvedValue([
    { name: 'a.txt', path: '/a.txt', is_dir: false, size: 3, modified: '' },
  ])
  wrapper = mount(FilesPage, { props: { resourceId: 'res-1' }, attachTo: document.body })
  await flushPromises()
  return wrapper
}

function confirmDialogExists(): boolean {
  return document.body.textContent?.includes('files.confirmDelete') ?? false
}

beforeEach(() => {
  vi.clearAllMocks()
  mockConnect.mockResolvedValue('test-session-123')
  mockListFiles.mockResolvedValue([])
  mockMkdir.mockResolvedValue(undefined)
  mockDeleteFile.mockResolvedValue(undefined)
  promptMock = vi.fn().mockReturnValue('docs')
  vi.stubGlobal('prompt', promptMock)
})

afterEach(() => {
  wrapper?.unmount()
  wrapper = undefined
  for (const w of extraWrappers) w.unmount()
  extraWrappers.length = 0
  vi.unstubAllGlobals()
  document.body.innerHTML = ''
})

describe('FilesPage shortcuts', () => {
  it('F7 creates a folder in the active panel', async () => {
    const w = await mountConnected()
    const before = mockListFiles.mock.calls.length

    press(w.element, 'F7')

    expect(promptMock).toHaveBeenCalledWith('files.folderNamePrompt')
    expect(mockMkdir).toHaveBeenCalledWith('test-session-123', '/docs')
    await flushPromises()
    expect(mockListFiles.mock.calls.length).toBe(before + 1)
  })

  it('F8 opens the delete confirmation for the selected item', async () => {
    const w = await mountConnected()
    await w.find('.fr:not(.fh)').trigger('click')
    expect(confirmDialogExists()).toBe(false)

    press(w.element, 'F8')
    await flushPromises()
    expect(confirmDialogExists()).toBe(true)
  })

  it('Delete opens the delete confirmation and confirms through the existing flow', async () => {
    const w = await mountConnected()
    await w.find('.fr:not(.fh)').trigger('click')

    press(w.element, 'Delete')
    await flushPromises()
    expect(confirmDialogExists()).toBe(true)

    const confirmBtn = [...document.body.querySelectorAll('button')]
      .find(b => b.textContent?.includes('files.delete'))
    expect(confirmBtn).toBeTruthy()
    confirmBtn!.click()
    await flushPromises()

    expect(mockDeleteFile).toHaveBeenCalledWith('test-session-123', '/a.txt')
  })

  it('F8 without a selection does not open the confirmation', async () => {
    const w = await mountConnected()
    press(w.element, 'F8')
    await flushPromises()
    expect(confirmDialogExists()).toBe(false)
  })

  it('Ctrl+R refreshes the file lists', async () => {
    const w = await mountConnected()
    const before = mockListFiles.mock.calls.length

    const event = press(w.element, 'r', { ctrlKey: true })
    expect(event.defaultPrevented).toBe(true)
    expect(mockListFiles.mock.calls.length).toBe(before + 2)
  })

  it('shortcuts are not hijacked while an input is focused', async () => {
    const w = await mountConnected()
    const input = document.createElement('input')
    w.element.appendChild(input)
    const before = mockListFiles.mock.calls.length

    press(input, 'F7')
    press(input, 'F8')
    press(input, 'Delete')
    press(input, 'r', { ctrlKey: true })

    expect(promptMock).not.toHaveBeenCalled()
    expect(mockMkdir).not.toHaveBeenCalled()
    expect(confirmDialogExists()).toBe(false)
    expect(mockListFiles.mock.calls.length).toBe(before)
  })

  it('keeps F2 rename behavior working outside inputs', async () => {
    const w = await mountConnected()
    await w.find('.fr:not(.fh)').trigger('click')

    press(w.element, 'F2')
    await flushPromises()

    expect(w.find('.fp-rename-input').exists()).toBe(true)
  })

  it('reports mkdir failures through a toast', async () => {
    mockMkdir.mockRejectedValue(new Error('EACCES'))
    const w = await mountConnected()

    press(w.element, 'F7')
    await flushPromises()

    expect(mockMkdir).toHaveBeenCalledTimes(1)
    expect(document.body.textContent).toContain('files.createFolderFailed')
  })

  it('reacts once, in the instance owning the focused pane', async () => {
    const ctx = buildPaneCtx(
      [{ id: 'pane-1', tabId: 'tab-a' }, { id: 'pane-2', tabId: 'tab-b' }],
      'pane-1',
      'tab-b',
      { label: 'SFTP', protocol: 'sftp' },
    )
    mountFilesInPane('res-1', 'tab-a', ctx)
    mountFilesInPane('res-2', 'tab-b', ctx)
    await flushPromises()

    press(document.body, 'F7')
    expect(promptMock).toHaveBeenCalledTimes(1)
    await flushPromises()
    expect(mockMkdir).toHaveBeenCalledTimes(1)

    const before = mockListFiles.mock.calls.length
    press(document.body, 'r', { ctrlKey: true })
    await flushPromises()
    // Only the focused instance refreshes its two panels.
    expect(mockListFiles.mock.calls.length).toBe(before + 2)
  })

  it('falls back to the active tab when the focused pane is empty', async () => {
    const ctx = buildPaneCtx(
      [{ id: 'pane-1', tabId: 'tab-a' }, { id: 'pane-2', tabId: 'tab-b' }],
      'pane-empty',
      'tab-a',
      { label: 'SFTP', protocol: 'sftp' },
    )
    mountFilesInPane('res-1', 'tab-a', ctx)
    mountFilesInPane('res-2', 'tab-b', ctx)
    await flushPromises()

    press(document.body, 'F7')
    expect(promptMock).toHaveBeenCalledTimes(1)
    await flushPromises()
    expect(mockMkdir).toHaveBeenCalledTimes(1)

    const before = mockListFiles.mock.calls.length
    press(document.body, 'r', { ctrlKey: true })
    await flushPromises()
    expect(mockListFiles.mock.calls.length).toBe(before + 2)
  })
})
