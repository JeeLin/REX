import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount, type VueWrapper } from '@vue/test-utils'
import { nextTick } from 'vue'

// i18n stub: returns the key itself.
vi.mock('vue-i18n', () => ({ useI18n: () => ({ t: (k: string) => k }) }))

// TerminalSearch.vue only imports the SearchAddon *type* (`import type`),
// which is erased at compile time — so no addon mock is needed here. Each
// pane gets a hand-rolled addon double instead.
import TerminalSearch from '../TerminalSearch.vue'

type ResultEvent = { resultIndex: number; resultCount: number }

function makeAddon() {
  const listeners: Array<(e: ResultEvent) => void> = []
  return {
    findNext: vi.fn(() => true),
    findPrevious: vi.fn(() => true),
    clearDecorations: vi.fn(),
    dispose: vi.fn(),
    onDidChangeResults(cb: (e: ResultEvent) => void) {
      listeners.push(cb)
      return {
        dispose: () => {
          const i = listeners.indexOf(cb)
          if (i >= 0) listeners.splice(i, 1)
        },
      }
    },
    emitResults(e: ResultEvent) {
      listeners.forEach((cb) => cb(e))
    },
  }
}

let wrappers: VueWrapper[] = []

function mountPane(visible: boolean, addon: ReturnType<typeof makeAddon>) {
  const w = mount(TerminalSearch, {
    props: { visible, searchAddon: addon as never },
    attachTo: document.body,
  })
  wrappers.push(w)
  return w
}

beforeEach(() => {
  wrappers = []
})

afterEach(() => {
  for (const w of wrappers) w.unmount()
  document.body.innerHTML = ''
})

describe('TerminalSearch', () => {
  // 回归：旧实现用 document.querySelector('.ts-input') + setTimeout(50ms)
  // 聚焦，而 WorkspaceTerminal 经 PaneNode → PaneLeaf 可多开同时挂载。
  // 全局选择器命中 DOM 里第一个搜索框 → 在 B 窗格按 Ctrl+F，焦点跳到 A 窗格。
  it('多窗格并存时，聚焦本窗格自己的搜索框而非 DOM 里第一个', async () => {
    const addonA = makeAddon()
    const addonB = makeAddon()

    const paneA = mountPane(true, addonA) // A 已打开 → 其 input 先进入 DOM
    await nextTick()
    const inputA = paneA.find('input.ts-input').element as HTMLInputElement
    expect(inputA).toBeTruthy()

    // B 窗格此时才打开：旧实现会聚焦先进入 DOM 的 A
    const paneB = mountPane(false, addonB)
    await paneB.setProps({ visible: true })
    await nextTick()
    await nextTick()

    const inputB = paneB.find('input.ts-input').element as HTMLInputElement

    expect(document.activeElement).toBe(inputB)
    expect(document.activeElement).not.toBe(inputA)
  })

  it('首次打开即聚焦搜索框（无需先存在其它搜索框）', async () => {
    const addon = makeAddon()
    const pane = mountPane(false, addon)
    await pane.setProps({ visible: true })
    await nextTick()
    await nextTick()

    expect(document.activeElement).toBe(pane.find('input.ts-input').element)
  })

  it('显示「第 n/N 个」匹配计数', async () => {
    const addon = makeAddon()
    const pane = mountPane(true, addon)
    await nextTick()
    await pane.find('input').setValue('docker')

    addon.emitResults({ resultIndex: 0, resultCount: 3 })
    await nextTick()

    expect(pane.find('.ts-count').text()).toBe('1/3')

    addon.emitResults({ resultIndex: 2, resultCount: 3 })
    await nextTick()
    expect(pane.find('.ts-count').text()).toBe('3/3')
  })

  it('resultIndex 为 -1（超过 highlightLimit）时显示 N+', async () => {
    const addon = makeAddon()
    const pane = mountPane(true, addon)
    await nextTick()
    await pane.find('input').setValue('x')

    addon.emitResults({ resultIndex: -1, resultCount: 1200 })
    await nextTick()

    expect(pane.find('.ts-count').text()).toBe('1200+')
  })

  it('无匹配时走 i18n 的 noResult 词条', async () => {
    const addon = makeAddon()
    const pane = mountPane(true, addon)
    await nextTick()
    await pane.find('input').setValue('nothing')

    addon.emitResults({ resultIndex: -1, resultCount: 0 })
    await nextTick()

    // i18n stub 原样返回 key
    expect(pane.find('.ts-count').text()).toBe('terminal.search.noResult')
  })

  it('输入变化即触发搜索；清空输入则清除高亮', async () => {
    const addon = makeAddon()
    const pane = mountPane(true, addon)
    await nextTick()

    await pane.find('input').setValue('pull')
    expect(addon.findNext).toHaveBeenCalledWith('pull', expect.anything())

    await pane.find('input').setValue('')
    expect(addon.clearDecorations).toHaveBeenCalled()
  })

  it('Enter 下一个 / Shift+Enter 上一个，且不穿透到 PTY', async () => {
    const addon = makeAddon()
    const pane = mountPane(true, addon)
    await nextTick()
    await pane.find('input').setValue('log')

    await pane.find('input').trigger('keydown', { key: 'Enter' })
    expect(addon.findNext).toHaveBeenCalled()

    await pane.find('input').trigger('keydown', { key: 'Enter', shiftKey: true })
    expect(addon.findPrevious).toHaveBeenCalled()
  })

  it('Escape 关闭并清空', async () => {
    const addon = makeAddon()
    const pane = mountPane(true, addon)
    await nextTick()
    await pane.find('input').setValue('abc')

    await pane.find('input').trigger('keydown', { key: 'Escape' })

    expect(pane.emitted('close')).toBeTruthy()
    expect(addon.clearDecorations).toHaveBeenCalled()
    expect((pane.find('input').element as HTMLInputElement).value).toBe('')
  })

  it('Aa / W / .* 开关切换后用当前词重搜，保证高亮与选项一致', async () => {
    const addon = makeAddon()
    const pane = mountPane(true, addon)
    await nextTick()
    await pane.find('input').setValue('abc')

    addon.findNext.mockClear()
    const caseBtn = pane.findAll('button').find((b) => b.text() === 'Aa')!
    await caseBtn.trigger('click')

    expect(addon.findNext).toHaveBeenCalledWith('abc', expect.objectContaining({ caseSensitive: true }))
  })
})
