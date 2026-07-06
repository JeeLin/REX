import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import TerminalMobileToolbar from '../TerminalMobileToolbar.vue'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}))

describe('TerminalMobileToolbar', () => {
  const createTerminalMock = () => ({
    focus: vi.fn(),
    textarea: {
      dispatchEvent: vi.fn(),
    },
  })

  it('renders all direction key buttons', () => {
    const terminal = createTerminalMock()
    const wrapper = mount(TerminalMobileToolbar, {
      props: { terminal, visible: true },
    })
    const dirButtons = wrapper.findAll('.direction-keys .toolbar-btn')
    // ↑↓←→ + Tab + Enter + ^C + ^L = 8 buttons + 3 separators
    expect(dirButtons.length).toBe(8)
  })

  it('renders function key buttons', () => {
    const terminal = createTerminalMock()
    const wrapper = mount(TerminalMobileToolbar, {
      props: { terminal, visible: true },
    })
    const funcButtons = wrapper.findAll('.function-keys .toolbar-btn')
    // 历史 + 粘贴 + A- + A+ + 更多 = 5 buttons + 2 separators
    expect(funcButtons.length).toBe(5)
  })

  it('does not render when visible is false', () => {
    const terminal = createTerminalMock()
    const wrapper = mount(TerminalMobileToolbar, {
      props: { terminal, visible: false },
    })
    expect(wrapper.find('.mobile-toolbar').exists()).toBe(false)
  })

  it('renders when visible is true', () => {
    const terminal = createTerminalMock()
    const wrapper = mount(TerminalMobileToolbar, {
      props: { terminal, visible: true },
    })
    expect(wrapper.find('.mobile-toolbar').exists()).toBe(true)
  })

  it('emits openHistory when history button clicked', async () => {
    const terminal = createTerminalMock()
    const wrapper = mount(TerminalMobileToolbar, {
      props: { terminal, visible: true },
    })
    const historyBtn = wrapper.findAll('.function-keys .toolbar-btn').at(0)
    await historyBtn?.trigger('click')
    expect(wrapper.emitted('openHistory')).toBeTruthy()
  })

  it('emits openPaste when paste button clicked', async () => {
    const terminal = createTerminalMock()
    const wrapper = mount(TerminalMobileToolbar, {
      props: { terminal, visible: true },
    })
    const pasteBtn = wrapper.findAll('.function-keys .toolbar-btn').at(1)
    await pasteBtn?.trigger('click')
    expect(wrapper.emitted('openPaste')).toBeTruthy()
  })

  it('emits fontSizeChange with -1 when A- clicked', async () => {
    const terminal = createTerminalMock()
    const wrapper = mount(TerminalMobileToolbar, {
      props: { terminal, visible: true },
    })
    const smallerBtn = wrapper.findAll('.function-keys .toolbar-btn').at(2)
    await smallerBtn?.trigger('click')
    expect(wrapper.emitted('fontSizeChange')).toBeTruthy()
    expect(wrapper.emitted('fontSizeChange')![0]).toEqual([-1])
  })

  it('emits fontSizeChange with 1 when A+ clicked', async () => {
    const terminal = createTerminalMock()
    const wrapper = mount(TerminalMobileToolbar, {
      props: { terminal, visible: true },
    })
    const largerBtn = wrapper.findAll('.function-keys .toolbar-btn').at(3)
    await largerBtn?.trigger('click')
    expect(wrapper.emitted('fontSizeChange')).toBeTruthy()
    expect(wrapper.emitted('fontSizeChange')![0]).toEqual([1])
  })

  it('shows more menu when more button clicked', async () => {
    const terminal = createTerminalMock()
    const wrapper = mount(TerminalMobileToolbar, {
      props: { terminal, visible: true },
    })
    const moreBtn = wrapper.findAll('.function-keys .toolbar-btn').at(4)
    await moreBtn?.trigger('click')
    expect(wrapper.find('.more-menu').exists()).toBe(true)
  })

  it('shows correct i18n keys for buttons', () => {
    const terminal = createTerminalMock()
    const wrapper = mount(TerminalMobileToolbar, {
      props: { terminal, visible: true },
    })
    expect(wrapper.text()).toContain('terminal.mobile.history')
    expect(wrapper.text()).toContain('terminal.mobile.paste')
    expect(wrapper.text()).toContain('terminal.mobile.more')
  })
})
