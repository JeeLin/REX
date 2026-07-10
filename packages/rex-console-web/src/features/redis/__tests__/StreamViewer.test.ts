import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import StreamViewer from '../types/StreamViewer.vue'
import type { RedisValue } from '@/api/redis'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}))

describe('StreamViewer', () => {
  const mockValue: RedisValue = {
    type: 'Array',
    value: [
      {
        type: 'Array',
        value: [
          { type: 'Bulk', value: '1-0' },
          {
            type: 'Array',
            value: [
              { type: 'Bulk', value: 'name' },
              { type: 'Bulk', value: 'test1' },
              { type: 'Bulk', value: 'value' },
              { type: 'Bulk', value: '100' },
            ],
          },
        ],
      },
      {
        type: 'Array',
        value: [
          { type: 'Bulk', value: '2-0' },
          {
            type: 'Array',
            value: [
              { type: 'Bulk', value: 'name' },
              { type: 'Bulk', value: 'test2' },
            ],
          },
        ],
      },
    ],
  }

  const createWrapper = (props = {}) => {
    return mount(StreamViewer, {
      props: { value: mockValue, ...props },
    })
  }

  it('renders stream entries', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('name')
    expect(wrapper.text()).toContain('test1')
    expect(wrapper.text()).toContain('100')
    expect(wrapper.text()).toContain('test2')
  })

  it('renders empty state when no entries', () => {
    const wrapper = createWrapper({ value: { type: 'Array', value: [] } })
    expect(wrapper.text()).toContain('redis.value.selectKey')
  })

  it('displays entry IDs', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('1-0')
    expect(wrapper.text()).toContain('2-0')
  })

  it('shows message count', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('2')
  })
})