import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount, flushPromises, type VueWrapper } from '@vue/test-utils'
import { setActivePinia, createPinia } from 'pinia'
import type { Environment } from '@/api/environments'

// i18n stub: returns the key itself.
vi.mock('vue-i18n', () => ({ useI18n: () => ({ t: (k: string) => k }) }))

// xterm is a heavy DOM renderer; render semantics under test do not depend on it.
// Hoisted so assertions outside the mock factory can inspect what was written.
const MockTerminal = vi.hoisted(() => {
  const writes: string[] = []
  return {
    writes,
    reset() { writes.length = 0 },
  }
})
vi.mock('@xterm/xterm', () => {
  class MockTerminalClass {
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
    write(data: string) { MockTerminal.writes.push(data) }
    writeln() { /* no-op */ }
    focus() { /* no-op */ }
    dispose() { /* no-op */ }
    clear() { /* no-op */ }
    selectAll() { /* no-op */ }
    getSelection() { return '' }
  }
  return { Terminal: MockTerminalClass }
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
    agents_online: 0,
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
    MockTerminal.reset()
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

  // ── terminal.data 解码（子任务 2 候选根因 1）────────────────────────
  // 后端 `terminal_ws.rs` 对 Rust String 的 UTF-8 **原始字节**做 base64
  // （`base64::encode(data.as_bytes())`）；前端旧实现用 `atob()` 直接转
  // 字符串 —— atob 返回 binary string（每个字符码点 = 一个字节），xterm
  // `write()` 接到后会再做一次 UTF-8 编码，于是所有 ≥ 0x80 的字节被双重
  // 编码，中文/ANSI 符号全部乱码。正确做法是先还原字节再 TextDecoder。
  //
  // 如用 Buffer.from(x, 'base64').toString('utf8')（标准基线）构造输入，
  // 则新实现必须与它逐字节一致；旧 atob 实现在非 ASCII 输入上会偏离。
  describe('terminal.data UTF-8 decoding', () => {
    async function mountConnected() {
      wrapper = await mountTerminal('agent', 'direct')
      const ws = FakeWebSocket.instances.at(-1)!
      ws.simulateOpen()
      await flushPromises()
      return ws
    }

    it('纯 ASCII 帧不受改动影响', async () => {
      const ws = await mountConnected()
      ws.simulateMessage('terminal.data', {
        data: Buffer.from('hello world').toString('base64'),
      })
      await flushPromises()
      expect(MockTerminal.writes.at(-1)).toBe('hello world')
    })

    it('中文/全角符号不再乱码（逐字节与基线一致）', async () => {
      const expected = '进度 50% ████████░░ 中文 [33m'
      const ws = await mountConnected()
      ws.simulateMessage('terminal.data', {
        data: Buffer.from(expected, 'utf8').toString('base64'),
      })
      await flushPromises()
      expect(MockTerminal.writes.at(-1)).toBe(expected)
    })

    it('跨帧切割的多字节序列（emoji 拆半）不丢不乱', async () => {
      const expected = '拉取中🚀完成'
      const raw = Buffer.from(expected, 'utf8')
      const mid = raw.length - 2 // 把 🚀 的尾两字节切到第二帧
      const ws = await mountConnected()
      ws.simulateMessage('terminal.data', { data: raw.subarray(0, mid).toString('base64') })
      await flushPromises()
      ws.simulateMessage('terminal.data', { data: raw.subarray(mid).toString('base64') })
      await flushPromises()
      const tail = MockTerminal.writes.slice(-2).join('')
      expect(tail).toContain('拉取中')
      expect(tail).toContain('完成')
      // 旧实现下两帧各自双重编码，合起来既无「拉取中」也无「🚀」
      expect(tail).not.toContain('\uFFFD')
    })

    it('非法 base64 输入不让 onmessage 抛错，会话继续', async () => {
      const ws = await mountConnected()
      // jsdom 的 atob 比浏览器宽松（忽略非法字符而非抛错），故此处只断言
      // 「不抛错 + 会话继续」这一确定行为，不锁定丢弃了多少字节。
      expect(() =>
        ws.simulateMessage('terminal.data', { data: 'not-valid-base64!!!' }),
      ).not.toThrow()
      await flushPromises()

      // 后续正常帧照常渲染 → 会话未被这帧破坏
      ws.simulateMessage('terminal.data', {
        data: Buffer.from('still alive').toString('base64'),
      })
      await flushPromises()
      expect(MockTerminal.writes.at(-1)).toBe('still alive')
    })
  })
})
