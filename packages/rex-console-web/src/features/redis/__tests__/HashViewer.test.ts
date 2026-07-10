import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import HashViewer from '../types/HashViewer.vue'
import type { RedisValue } from '@/api/redis'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}))

describe('HashViewer', () => {
  const mockValue: RedisValue = {
    type: 'Array',
    value: [
      { type: 'Bulk', value: 'name' },
      { type: 'Bulk', value: 'test' },
      { type: 'Bulk', value: 'age' },
      { type: 'Bulk', value: '25' },
    ],
  }

  const createWrapper = (props = {}) => {
    return mount(HashViewer, {
      props: { value: mockValue, ...props },
    })
  }

  it('renders hash entries', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('name')
    expect(wrapper.text()).toContain('test')
    expect(wrapper.text()).toContain('age')
    expect(wrapper.text()).toContain('25')
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