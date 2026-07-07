import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import LoadingSpinner from '../LoadingSpinner.vue'

describe('LoadingSpinner', () => {
  it('renders spinner element', () => {
    const wrapper = mount(LoadingSpinner)
    expect(wrapper.find('.spinner').exists()).toBe(true)
  })

  it('applies md size class by default', () => {
    const wrapper = mount(LoadingSpinner)
    expect(wrapper.find('.size-md').exists()).toBe(true)
  })

  it('applies sm size class', () => {
    const wrapper = mount(LoadingSpinner, { props: { size: 'sm' } })
    expect(wrapper.find('.size-sm').exists()).toBe(true)
  })

  it('applies lg size class', () => {
    const wrapper = mount(LoadingSpinner, { props: { size: 'lg' } })
    expect(wrapper.find('.size-lg').exists()).toBe(true)
  })

  it('renders text when provided', () => {
    const wrapper = mount(LoadingSpinner, { props: { text: 'Loading...' } })
    expect(wrapper.find('.loading-text').exists()).toBe(true)
    expect(wrapper.find('.loading-text').text()).toBe('Loading...')
  })

  it('does not render text when not provided', () => {
    const wrapper = mount(LoadingSpinner)
    expect(wrapper.find('.loading-text').exists()).toBe(false)
  })
})
