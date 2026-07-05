import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import TabBar from '../TabBar.vue'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}))

vi.mock('../useTabs', () => ({
  useTabs: () => ({
    tabs: { value: [] },
    activeTabId: { value: null },
    activePanelIndex: { value: 0 },
    activateTab: vi.fn(),
    closeTab: vi.fn(),
    closeOtherTabs: vi.fn(),
    closeTabsRight: vi.fn(),
    closeTabsLeft: vi.fn(),
    closeAllTabs: vi.fn(),
    duplicateTab: vi.fn(),
    moveTabToPanel: vi.fn(),
    disconnectAll: vi.fn(),
    reorderTab: vi.fn(),
  }),
}))

vi.mock('@/composables/useContextMenu', () => ({
  useContextMenu: () => ({
    show: vi.fn(),
  }),
}))

describe('TabBar', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders the add button', () => {
    const wrapper = mount(TabBar, {
      props: { panelCount: 1 },
      slots: { right: '<div class="right-slot">Right</div>' },
    })
    expect(wrapper.find('.ws-tab-add').exists()).toBe(true)
  })

  it('emits newConnection when add button is clicked', async () => {
    const wrapper = mount(TabBar, {
      props: { panelCount: 1 },
      slots: { right: '<div />' },
    })
    await wrapper.find('.ws-tab-add').trigger('click')
    expect(wrapper.emitted('newConnection')).toBeTruthy()
  })

  it('renders right slot', () => {
    const wrapper = mount(TabBar, {
      props: { panelCount: 1 },
      slots: { right: '<div class="right-slot">Content</div>' },
    })
    expect(wrapper.find('.right-slot').text()).toBe('Content')
  })
})
