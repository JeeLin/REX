import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import FileMobileToolbar from '../FileMobileToolbar.vue'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}))

describe('FileMobileToolbar', () => {
  it('renders all action buttons', () => {
    const wrapper = mount(FileMobileToolbar, {
      props: { visible: true, selectedCount: 0 },
    })
    const buttons = wrapper.findAll('.toolbar-btn')
    // 上传 + 新建文件 + 新建文件夹 + 刷新 + 下载 + 删除 + 全选 = 7
    expect(buttons.length).toBe(7)
  })

  it('does not render when visible is false', () => {
    const wrapper = mount(FileMobileToolbar, {
      props: { visible: false, selectedCount: 0 },
    })
    expect(wrapper.find('.mobile-toolbar').exists()).toBe(false)
  })

  it('renders when visible is true', () => {
    const wrapper = mount(FileMobileToolbar, {
      props: { visible: true, selectedCount: 0 },
    })
    expect(wrapper.find('.mobile-toolbar').exists()).toBe(true)
  })

  it('emits upload when upload button clicked', async () => {
    const wrapper = mount(FileMobileToolbar, {
      props: { visible: true, selectedCount: 0 },
    })
    await wrapper.findAll('.toolbar-btn')[0]!.trigger('click')
    expect(wrapper.emitted('upload')).toBeTruthy()
  })

  it('emits newFile when new file button clicked', async () => {
    const wrapper = mount(FileMobileToolbar, {
      props: { visible: true, selectedCount: 0 },
    })
    await wrapper.findAll('.toolbar-btn')[1]!.trigger('click')
    expect(wrapper.emitted('newFile')).toBeTruthy()
  })

  it('emits newFolder when new folder button clicked', async () => {
    const wrapper = mount(FileMobileToolbar, {
      props: { visible: true, selectedCount: 0 },
    })
    await wrapper.findAll('.toolbar-btn')[2]!.trigger('click')
    expect(wrapper.emitted('newFolder')).toBeTruthy()
  })

  it('emits refresh when refresh button clicked', async () => {
    const wrapper = mount(FileMobileToolbar, {
      props: { visible: true, selectedCount: 0 },
    })
    await wrapper.findAll('.toolbar-btn')[3]!.trigger('click')
    expect(wrapper.emitted('refresh')).toBeTruthy()
  })

  it('disables download and delete when no selection', () => {
    const wrapper = mount(FileMobileToolbar, {
      props: { visible: true, selectedCount: 0 },
    })
    const buttons = wrapper.findAll('.toolbar-btn')
    // 下载 = index 4, 删除 = index 5
    expect(buttons[4]!.attributes('disabled')).toBeDefined()
    expect(buttons[5]!.attributes('disabled')).toBeDefined()
  })

  it('enables download and delete when items selected', () => {
    const wrapper = mount(FileMobileToolbar, {
      props: { visible: true, selectedCount: 2 },
    })
    const buttons = wrapper.findAll('.toolbar-btn')
    expect(buttons[4]!.attributes('disabled')).toBeUndefined()
    expect(buttons[5]!.attributes('disabled')).toBeUndefined()
  })

  it('emits selectAll when select all button clicked', async () => {
    const wrapper = mount(FileMobileToolbar, {
      props: { visible: true, selectedCount: 0 },
    })
    await wrapper.findAll('.toolbar-btn')[6]!.trigger('click')
    expect(wrapper.emitted('selectAll')).toBeTruthy()
  })

  it('shows correct i18n keys for buttons', () => {
    const wrapper = mount(FileMobileToolbar, {
      props: { visible: true, selectedCount: 0 },
    })
    expect(wrapper.text()).toContain('files.upload')
    expect(wrapper.text()).toContain('files.newFile')
    expect(wrapper.text()).toContain('files.newFolder')
    expect(wrapper.text()).toContain('files.refresh')
  })
})
