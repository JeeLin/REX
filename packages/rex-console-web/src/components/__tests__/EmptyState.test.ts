import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import EmptyState from '../EmptyState.vue'

describe('EmptyState', () => {
  it('renders title', () => {
    const wrapper = mount(EmptyState, {
      props: { title: 'No data found' },
    })
    expect(wrapper.find('.empty-title').text()).toBe('No data found')
  })

  it('renders icon when provided', () => {
    const wrapper = mount(EmptyState, {
      props: { title: 'Empty', icon: '📦' },
    })
    expect(wrapper.find('.empty-icon').exists()).toBe(true)
    expect(wrapper.find('.empty-icon').text()).toBe('📦')
  })

  it('does not render icon when not provided', () => {
    const wrapper = mount(EmptyState, {
      props: { title: 'Empty' },
    })
    expect(wrapper.find('.empty-icon').exists()).toBe(false)
  })

  it('renders hint when provided', () => {
    const wrapper = mount(EmptyState, {
      props: { title: 'Empty', hint: 'Try creating something' },
    })
    expect(wrapper.find('.empty-hint').text()).toBe('Try creating something')
  })

  it('does not render hint when not provided', () => {
    const wrapper = mount(EmptyState, {
      props: { title: 'Empty' },
    })
    expect(wrapper.find('.empty-hint').exists()).toBe(false)
  })

  it('renders action button when action is provided', () => {
    const handler = vi.fn()
    const wrapper = mount(EmptyState, {
      props: {
        title: 'Empty',
        action: { label: 'Create', handler },
      },
    })
    const btn = wrapper.find('button')
    expect(btn.exists()).toBe(true)
    expect(btn.text()).toBe('Create')
  })

  it('calls action handler when button is clicked', async () => {
    const handler = vi.fn()
    const wrapper = mount(EmptyState, {
      props: {
        title: 'Empty',
        action: { label: 'Create', handler },
      },
    })
    await wrapper.find('button').trigger('click')
    expect(handler).toHaveBeenCalledTimes(1)
  })

  it('does not render action button when action is not provided', () => {
    const wrapper = mount(EmptyState, {
      props: { title: 'Empty' },
    })
    expect(wrapper.find('button').exists()).toBe(false)
  })
})
