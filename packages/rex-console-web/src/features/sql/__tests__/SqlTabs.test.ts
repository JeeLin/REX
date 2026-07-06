import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import SqlTabs from '../SqlTabs.vue'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}))

vi.mock('@/composables/useContextMenu', () => ({
  useContextMenu: () => ({
    show: vi.fn(),
  }),
}))

describe('SqlTabs', () => {
  const baseTabs = [
    { id: 'tab-1', title: 'Query 1', queryId: null, subtitle: 'SELECT *' },
    { id: 'tab-2', title: 'Query 2', queryId: 'q-123', subtitle: 'INSERT INTO' },
  ]

  it('renders tab titles', () => {
    const wrapper = mount(SqlTabs, {
      props: { tabs: baseTabs, activeId: 'tab-1' },
    })
    const labels = wrapper.findAll('.tab-title')
    expect(labels.length).toBe(2)
    expect(labels[0].text()).toBe('Query 1')
    expect(labels[1].text()).toBe('Query 2')
  })

  it('shows unsaved indicator for tabs without queryId', () => {
    const wrapper = mount(SqlTabs, {
      props: { tabs: baseTabs, activeId: 'tab-1' },
    })
    const unsaved = wrapper.findAll('.tab-unsaved')
    // Only tab-1 has queryId=null, so one unsaved indicator
    expect(unsaved.length).toBe(1)
  })

  it('does not show unsaved indicator for saved tabs', () => {
    const savedTabs = [
      { id: 'tab-1', title: 'Saved', queryId: 'q-1', subtitle: undefined },
    ]
    const wrapper = mount(SqlTabs, {
      props: { tabs: savedTabs, activeId: 'tab-1' },
    })
    expect(wrapper.find('.tab-unsaved').exists()).toBe(false)
  })

  it('shows subtitle when provided', () => {
    const wrapper = mount(SqlTabs, {
      props: { tabs: baseTabs, activeId: 'tab-1' },
    })
    const subtitles = wrapper.findAll('.tab-subtitle')
    expect(subtitles.length).toBe(2)
    expect(subtitles[0].text()).toBe('SELECT *')
    expect(subtitles[1].text()).toBe('INSERT INTO')
  })

  it('does not show subtitle when undefined', () => {
    const tabs = [
      { id: 'tab-1', title: 'Query', queryId: null, subtitle: undefined },
    ]
    const wrapper = mount(SqlTabs, {
      props: { tabs, activeId: 'tab-1' },
    })
    expect(wrapper.find('.tab-subtitle').exists()).toBe(false)
  })

  it('applies active class to the active tab', () => {
    const wrapper = mount(SqlTabs, {
      props: { tabs: baseTabs, activeId: 'tab-2' },
    })
    const tabs = wrapper.findAll('.sql-tab')
    expect(tabs[0].classes()).not.toContain('active')
    expect(tabs[1].classes()).toContain('active')
  })

  it('emits select event on tab click', async () => {
    const wrapper = mount(SqlTabs, {
      props: { tabs: baseTabs, activeId: 'tab-1' },
    })
    await wrapper.findAll('.sql-tab')[1].trigger('click')
    expect(wrapper.emitted('select')).toEqual([['tab-2']])
  })

  it('emits close event on close button click', async () => {
    const wrapper = mount(SqlTabs, {
      props: { tabs: baseTabs, activeId: 'tab-1' },
    })
    await wrapper.findAll('.tab-close')[0].trigger('click')
    expect(wrapper.emitted('close')).toEqual([['tab-1']])
  })

  it('emits add event on plus button click', async () => {
    const wrapper = mount(SqlTabs, {
      props: { tabs: baseTabs, activeId: 'tab-1' },
    })
    await wrapper.find('.sql-tab-add').trigger('click')
    expect(wrapper.emitted('add')).toHaveLength(1)
  })

  it('hides close button when only one tab', () => {
    const wrapper = mount(SqlTabs, {
      props: { tabs: [baseTabs[0]], activeId: 'tab-1' },
    })
    expect(wrapper.find('.tab-close').exists()).toBe(false)
  })

  it('shows tab icon as saved when queryId exists', () => {
    const wrapper = mount(SqlTabs, {
      props: { tabs: baseTabs, activeId: 'tab-1' },
    })
    const icons = wrapper.findAll('.tab-icon')
    expect(icons[0].text()).toBe('📄') // unsaved
    expect(icons[1].text()).toBe('💾') // saved
  })
})
