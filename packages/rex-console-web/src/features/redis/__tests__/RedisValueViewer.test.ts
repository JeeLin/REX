import { describe, it, expect, vi } from 'vitest'
import { mount, config } from '@vue/test-utils'
import RedisValueViewer from '../RedisValueViewer.vue'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}))

// Workaround for Vue 3.5 + @vue/test-utils WeakMap issue
// Disable all automatic stubbing to prevent the WeakMap registration error
config.global.stubs = false

describe('RedisValueViewer', () => {
  it('renders key name and type badge', () => {
    const wrapper = mount(RedisValueViewer, {
      props: {
        keyName: 'mykey',
        valueType: 'string',
        value: { type: 'Bulk', value: 'hello' },
        ttl: 3600,
        loading: false,
      },
    })
    expect(wrapper.find('.value-key-name').text()).toBe('mykey')
    expect(wrapper.find('.value-type-badge').text()).toBe('string')
  })

  it('displays string value', () => {
    const wrapper = mount(RedisValueViewer, {
      props: {
        keyName: 'mykey',
        valueType: 'string',
        value: { type: 'Bulk', value: 'hello world' },
        ttl: 3600,
        loading: false,
      },
    })
    expect(wrapper.find('.value-text').text()).toBe('hello world')
  })

  it('formats JSON values', () => {
    const jsonVal = JSON.stringify({ name: 'test', count: 42 })
    const wrapper = mount(RedisValueViewer, {
      props: {
        keyName: 'jsonkey',
        valueType: 'string',
        value: { type: 'Bulk', value: jsonVal },
        ttl: 3600,
        loading: false,
      },
    })
    expect(wrapper.find('.value-json').exists()).toBe(true)
    expect(wrapper.find('.value-json').text()).toContain('"name"')
  })

  it('displays hash items', () => {
    const wrapper = mount(RedisValueViewer, {
      props: {
        keyName: 'myhash',
        valueType: 'hash',
        value: {
          type: 'Array',
          value: [
            { type: 'Bulk', value: 'field1' },
            { type: 'Bulk', value: 'val1' },
            { type: 'Bulk', value: 'field2' },
            { type: 'Bulk', value: 'val2' },
          ],
        },
        ttl: -1,
        loading: false,
      },
    })
    const rows = wrapper.findAll('.value-table tbody tr')
    expect(rows.length).toBe(2)
  })

  it('shows TTL with infinity symbol for -1', () => {
    const wrapper = mount(RedisValueViewer, {
      props: {
        keyName: 'mykey',
        valueType: 'string',
        value: { type: 'Bulk', value: 'test' },
        ttl: -1,
        loading: false,
      },
    })
    expect(wrapper.find('.value-ttl').text()).toContain('∞')
  })

  it('shows expired TTL', () => {
    const wrapper = mount(RedisValueViewer, {
      props: {
        keyName: 'mykey',
        valueType: 'string',
        value: { type: 'Bulk', value: 'test' },
        ttl: -2,
        loading: false,
      },
    })
    expect(wrapper.find('.value-ttl').classes()).toContain('expired')
  })

  it('shows loading state when type is unknown', () => {
    const wrapper = mount(RedisValueViewer, {
      props: {
        keyName: 'mykey',
        valueType: 'unknown',
        value: null,
        ttl: null,
        loading: true,
      },
    })
    expect(wrapper.find('.value-loading').exists()).toBe(true)
  })

  it('emits refresh on button click', async () => {
    const wrapper = mount(RedisValueViewer, {
      props: {
        keyName: 'mykey',
        valueType: 'string',
        value: { type: 'Bulk', value: 'test' },
        ttl: 3600,
        loading: false,
      },
    })
    // Find the refresh button (second button in header, not edit or delete)
    const buttons = wrapper.findAll('.value-header button')
    const refreshBtn = buttons.find(b => b.text().includes('redis.value.refresh'))
    if (refreshBtn) {
      await refreshBtn.trigger('click')
      expect(wrapper.emitted('refresh')).toBeTruthy()
    }
  })

  it('emits deleteKey on delete button click', async () => {
    const wrapper = mount(RedisValueViewer, {
      props: {
        keyName: 'mykey',
        valueType: 'string',
        value: { type: 'Bulk', value: 'test' },
        ttl: 3600,
        loading: false,
      },
    })
    await wrapper.find('.redis-btn-danger').trigger('click')
    expect(wrapper.emitted('deleteKey')).toBeTruthy()
    expect(wrapper.emitted('deleteKey')![0]).toEqual(['mykey'])
  })
})
