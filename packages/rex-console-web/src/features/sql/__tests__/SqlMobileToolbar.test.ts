import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import SqlMobileToolbar from '../SqlMobileToolbar.vue'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}))

describe('SqlMobileToolbar', () => {
  it('renders all action buttons', () => {
    const wrapper = mount(SqlMobileToolbar, {
      props: { visible: true },
    })
    const buttons = wrapper.findAll('.toolbar-btn')
    // 执行 + 格式化 + 清空 + 保存 + 历史 + 全局查询 + 更多 = 7
    expect(buttons.length).toBe(7)
  })

  it('does not render when visible is false', () => {
    const wrapper = mount(SqlMobileToolbar, {
      props: { visible: false },
    })
    expect(wrapper.find('.mobile-toolbar').exists()).toBe(false)
  })

  it('renders when visible is true', () => {
    const wrapper = mount(SqlMobileToolbar, {
      props: { visible: true },
    })
    expect(wrapper.find('.mobile-toolbar').exists()).toBe(true)
  })

  it('emits execute when execute button clicked', async () => {
    const wrapper = mount(SqlMobileToolbar, {
      props: { visible: true },
    })
    await wrapper.findAll('.toolbar-btn')[0].trigger('click')
    expect(wrapper.emitted('execute')).toBeTruthy()
  })

  it('emits format when format button clicked', async () => {
    const wrapper = mount(SqlMobileToolbar, {
      props: { visible: true },
    })
    await wrapper.findAll('.toolbar-btn')[1].trigger('click')
    expect(wrapper.emitted('format')).toBeTruthy()
  })

  it('emits clear when clear button clicked', async () => {
    const wrapper = mount(SqlMobileToolbar, {
      props: { visible: true },
    })
    await wrapper.findAll('.toolbar-btn')[2].trigger('click')
    expect(wrapper.emitted('clear')).toBeTruthy()
  })

  it('emits save when save button clicked', async () => {
    const wrapper = mount(SqlMobileToolbar, {
      props: { visible: true },
    })
    await wrapper.findAll('.toolbar-btn')[3].trigger('click')
    expect(wrapper.emitted('save')).toBeTruthy()
  })

  it('emits history when history button clicked', async () => {
    const wrapper = mount(SqlMobileToolbar, {
      props: { visible: true },
    })
    await wrapper.findAll('.toolbar-btn')[4].trigger('click')
    expect(wrapper.emitted('history')).toBeTruthy()
  })

  it('emits globalQuery when global query button clicked', async () => {
    const wrapper = mount(SqlMobileToolbar, {
      props: { visible: true },
    })
    await wrapper.findAll('.toolbar-btn')[5].trigger('click')
    expect(wrapper.emitted('globalQuery')).toBeTruthy()
  })

  it('shows more menu and emits openQuery', async () => {
    const dispatchSpy = vi.spyOn(window, 'dispatchEvent')
    const wrapper = mount(SqlMobileToolbar, {
      props: { visible: true },
    })
    // Click more button (index 6)
    await wrapper.findAll('.toolbar-btn')[6].trigger('click')
    // More menu should appear
    expect(wrapper.find('.more-menu').exists()).toBe(true)
    // Click openQuery in more menu
    await wrapper.find('.more-menu-item').trigger('click')
    expect(dispatchSpy).toHaveBeenCalledWith(
      expect.objectContaining({
        detail: 'openQuery',
      }),
    )
    dispatchSpy.mockRestore()
  })

  it('shows correct i18n keys for buttons', () => {
    const wrapper = mount(SqlMobileToolbar, {
      props: { visible: true },
    })
    expect(wrapper.text()).toContain('sql.mobile.execute')
    expect(wrapper.text()).toContain('sql.mobile.format')
    expect(wrapper.text()).toContain('sql.mobile.clear')
    expect(wrapper.text()).toContain('sql.mobile.save')
    expect(wrapper.text()).toContain('sql.mobile.history')
    expect(wrapper.text()).toContain('sql.mobile.globalQuery')
    expect(wrapper.text()).toContain('sql.mobile.more')
  })
})
