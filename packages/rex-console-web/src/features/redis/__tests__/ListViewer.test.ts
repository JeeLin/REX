import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import ListViewer from '../types/ListViewer.vue'
import type { RedisValue } from '@/api/redis'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}))

describe('ListViewer', () => {
  const mockValue: RedisValue = {
    type: 'Array',
    value: [
      { type: 'Bulk', value: 'value1' },
      { type: 'Bulk', value: 'value2' },
      { type: 'Bulk', value: 'value3' },
    ],
  }

  const createWrapper = (props = {}) => {
    return mount(ListViewer, {
      props: { value: mockValue, ...props },
    })
  }

  it('renders list entries', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('value1')
    expect(wrapper.text()).toContain('value2')
    expect(wrapper.text()).toContain('value3')
  })

  it('renders empty state when no entries', () => {
    const wrapper = createWrapper({ value: { type: 'Array', value: [] } })
    expect(wrapper.text()).toContain('redis.value.selectKey')
  })

  it('renders empty state when value is null', () => {
    const wrapper = createWrapper({ value: null })
    expect(wrapper.text()).toContain('redis.value.selectKey')
  })
})