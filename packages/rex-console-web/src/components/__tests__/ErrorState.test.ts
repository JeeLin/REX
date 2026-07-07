import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import ErrorState from '../ErrorState.vue'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => {
      const messages: Record<string, string> = {
        'common.retry': 'Retry',
      }
      return messages[key] || key
    },
  }),
}))

describe('ErrorState', () => {
  it('renders error message', () => {
    const wrapper = mount(ErrorState, {
      props: { message: 'Something went wrong' },
    })
    expect(wrapper.find('.error-message').text()).toBe('Something went wrong')
  })

  it('renders warning icon', () => {
    const wrapper = mount(ErrorState, {
      props: { message: 'Error' },
    })
    expect(wrapper.find('.error-icon').exists()).toBe(true)
    expect(wrapper.find('.error-icon').text()).toBe('⚠')
  })

  it('renders retry button when retry callback is provided', () => {
    const retry = vi.fn()
    const wrapper = mount(ErrorState, {
      props: { message: 'Error', retry },
    })
    const btn = wrapper.find('button')
    expect(btn.exists()).toBe(true)
    expect(btn.text()).toBe('Retry')
  })

  it('calls retry when button is clicked', async () => {
    const retry = vi.fn()
    const wrapper = mount(ErrorState, {
      props: { message: 'Error', retry },
    })
    await wrapper.find('button').trigger('click')
    expect(retry).toHaveBeenCalledTimes(1)
  })

  it('does not render retry button when retry is not provided', () => {
    const wrapper = mount(ErrorState, {
      props: { message: 'Error' },
    })
    expect(wrapper.find('button').exists()).toBe(false)
  })
})
