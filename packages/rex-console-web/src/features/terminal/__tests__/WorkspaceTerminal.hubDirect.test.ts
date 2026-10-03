import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount, flushPromises, type VueWrapper } from '@vue/test-utils'
import { setActivePinia, createPinia } from 'pinia'
import type { Environment } from '@/api/environments'

// i18n stub: returns the key itself.
vi.mock('vue-i18n', () => ({ useI18n: () => ({ t: (k: string) => k }) }))

// xterm is a heavy DOM renderer; render semantics under test do not depend on it.
vi.mock('@xterm/xterm', () => {
  class MockTerminal {
    cols = 80
    rows = 24
    options: Record<string, unknown>
    unicode = { activeVersion: '0' }
    buffer = { active: 'normal', normal: 'normal' }

    constructor(options: Record<string, unknown> = {}) {
      this.options = options
    }

    loadAddon() { /* no-op */ }
    open() { /* no-op */ }
    attachCustomKeyEventHandler() { /* no-op */ }
    onData() { /* no-op */ }
    onResize() { /* no-op */ }
    write() { /* no-op */ }
    writeln() { /* no-op */ }
    focus() { /* no-op */ }
    dispose() { /* no-op */ }
    clear() { /* no-op */ }
    selectAll() { /* no-op */ }
    getSelection() { return '' }
  }
  return { Terminal: MockTerminal }
})
vi.mock('@xterm/addon-fit', () => ({ FitAddon: class { fit() { /* no-op */ } } }))
vi.mock('@xterm/addon-search', () => ({
  SearchAddon: class {
    findNext() { return false }
    findPrevious() { return false }
    clearDecorations() { /* no-op */ }
    dispose() { /* no-op */ }
  },
}))
vi.mock('@xterm/addon-unicode11', () => ({ Unicode11Addon: class { /* no-op */ } }))

vi.mock('@/components/ui/Modal.vue', () => ({
  default: { name: 'ModalStub', template: '<div class="modal-stub" />', props: ['modelValue', 'title', 'width'] },
}))
const toastPush = vi.hoisted(() => vi.fn())
vi.mock('@/components/ui/Toast.vue', () => ({
  default: {
    name: 'ToastStub',
    methods: { push: toastPush },
    template: '<div class="toast-stub" />',
  },
}))

import WorkspaceTerminal from '../WorkspaceTerminal.vue'
import { useAppStore } from '@/stores/app'
import { useEnvironmentsStore } from '@/stores/environments'

/** Controllable WebSocket double: handlers are assigned by the component after construction. */
class FakeWebSocket {
  static instances: FakeWebSocket[] = []
  static CONNECTING = 0
  static OPEN = 1
  static CLOSING = 2
  static CLOSED = 3

  url: string
  readyState = 0
  onopen: (() => void) | null = null
  onclose: (() => void) | null = null
  onerror: (() => void) | null = null
  onmessage: ((event: MessageEvent) => void) | null = null

  constructor(url: string) {
    this.url = url
    FakeWebSocket.instances.push(this)
  }

  send(_data: string) { /* no-op */ }
  close() { this.readyState = FakeWebSocket.CLOSED }

  /** Handshake succeeded. */
  simulateOpen() {
    this.readyState = FakeWebSocket.OPEN
    this.onopen?.()
  }

  /** Deliver a server→client JSON message to the component's onmessage handler. */
  simulateMessage(type: string, payload: Record<string, unknown> = {}) {
    this.onmessage?.({ data: JSON.stringify({ type, payload }) } as MessageEvent)
  }

  /** Trigger the transport-level error handler (ws.onerror). */
  simulateError() {
    this.onerror?.()
  }

  /** Transport failure before any session was opened (e.g. server refused the WS). */
  simulateFailBeforeOpen() {
    this.readyState = FakeWebSocket.CLOSED
    this.onclose?.()
  }

  /** Session was open, then dropped. */
  simulateCloseAfterOpen() {
    this.readyState = FakeWebSocket.CLOSED
    this.onclose?.()
  }
}

function makeEnv(id: string, connectionMode: string): Environment {
  return {
    id,
    name: `Env ${id}`,
    description: '',
    connection_mode: connectionMode,
    resource_count: 1,
    agent_status: null,
    registration_token: 'tok',
    created_at: '',
    updated_at: '',
  }
}

async function mountTerminal(appMode: 'hub' | 'agent', connectionMode: string): Promise<VueWrapper> {
  const appStore = useAppStore()
  const envStore = useEnvironmentsStore()
  appStore.mode = appMode
  envStore.environments = [makeEnv('env-1', connectionMode)]

  const wrapper = mount(WorkspaceTerminal, {
    props: { tabId: 'tab-1', resourceId: 'r1', environmentId: 'env-1', protocol: 'ssh' },
  })
  // onMounted awaits nextTick before initTerminal()/connectSession().
  await flushPromises()
  return wrapper
}

describe('WorkspaceTerminal hub-direct failure hint', () => {
  let wrapper: VueWrapper | null = null

  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    FakeWebSocket.instances = []
    vi.stubGlobal('WebSocket', FakeWebSocket)
  })

  afterEach(() => {
    wrapper?.unmount()
    wrapper = null
    vi.unstubAllGlobals()
  })

  it('shows the hub-direct hint in the failure overlay when connectFailed && hubDirect', async () => {
    wrapper = await mountTerminal('agent', 'direct')
    const ws = FakeWebSocket.instances.at(-1)!
    expect(ws).toBeTruthy()

    ws.simulateFailBeforeOpen()
    await flushPromises()

    expect(wrapper.find('.wt-overlay').exists()).toBe(true)
    const hint = wrapper.find('.wt-overlay-hint')
    expect(hint.exists()).toBe(true)
    expect(hint.text()).toContain('terminal.hintHubDirect')
  })

  it('hides the hint when the failed env is not hub-direct (agent connection_mode)', async () => {
    wrapper = await mountTerminal('agent', 'agent')
    const ws = FakeWebSocket.instances.at(-1)!

    ws.simulateFailBeforeOpen()
    await flushPromises()

    expect(wrapper.find('.wt-overlay').exists()).toBe(true)
    expect(wrapper.find('.wt-overlay-hint').exists()).toBe(false)
  })

  it('hides the hint when failed in hub (non-agent) mode', async () => {
    wrapper = await mountTerminal('hub', 'direct')
    const ws = FakeWebSocket.instances.at(-1)!

    ws.simulateFailBeforeOpen()
    await flushPromises()

    expect(wrapper.find('.wt-overlay').exists()).toBe(true)
    expect(wrapper.find('.wt-overlay-hint').exists()).toBe(false)
  })

  it('keeps the hint hidden while connected and after a clean close of an opened session', async () => {
    wrapper = await mountTerminal('agent', 'direct')
    const ws = FakeWebSocket.instances.at(-1)!

    ws.simulateOpen()
    await flushPromises()
    // Session is up — no failure overlay at all.
    expect(wrapper.find('.wt-overlay').exists()).toBe(false)
    expect(wrapper.find('.wt-overlay-hint').exists()).toBe(false)

    ws.simulateCloseAfterOpen()
    await flushPromises()
    // Overlay returns after the drop, but connectFailed stays false → no hub-direct hint.
    expect(wrapper.find('.wt-overlay').exists()).toBe(true)
    expect(wrapper.find('.wt-overlay-hint').exists()).toBe(false)
  })

  it('pushes an error toast with code/message when terminal.error arrives', async () => {
    toastPush.mockClear()
    wrapper = await mountTerminal('hub', 'direct')
    const ws = FakeWebSocket.instances.at(-1)!
    ws.simulateOpen()
    await flushPromises()

    ws.simulateMessage('terminal.error', { code: 'SSH_FAIL', message: 'boom' })
    await flushPromises()

    expect(toastPush).toHaveBeenCalledWith('boom', 'error')
  })

  it('pushes an error toast on ws.onerror', async () => {
    toastPush.mockClear()
    wrapper = await mountTerminal('hub', 'direct')
    const ws = FakeWebSocket.instances.at(-1)!
    ws.simulateOpen()
    await flushPromises()

    ws.simulateError()
    await flushPromises()

    expect(toastPush).toHaveBeenCalledWith('terminal.wsError', 'error')
  })
})
