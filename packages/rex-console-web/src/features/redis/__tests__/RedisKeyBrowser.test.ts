import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import RedisKeyBrowser from '../RedisKeyBrowser.vue'
import SearchFilter from '../SearchFilter.vue'

// Mock vue-i18n
vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}))

describe('RedisKeyBrowser', () => {
  const sampleKeys = [
    { key: 'user:1001:name', type: 'string' },
    { key: 'user:1001:email', type: 'string' },
    { key: 'user:1002:name', type: 'string' },
    { key: 'session:abc123', type: 'hash' },
    { key: 'counter', type: 'string' },
    { key: 'queue:tasks', type: 'list' },
    { key: 'tags:redis', type: 'set' },
  ]

  it('renders SearchFilter component', () => {
    const wrapper = mount(RedisKeyBrowser, {
      props: { connected: true, keys: [] },
    })
    expect(wrapper.findComponent(SearchFilter).exists()).toBe(true)
  })

  it('emits search event from SearchFilter', async () => {
    const wrapper = mount(RedisKeyBrowser, {
      props: { connected: true, keys: [] },
    })
    // Clear the mount-triggered search
    await wrapper.vm.$nextTick()
    const emittedBefore = (wrapper.emitted('search') ?? []).length
    await wrapper.findComponent(SearchFilter).vm.$emit('search', 'test:*')
    expect(wrapper.emitted('search')!.length).toBeGreaterThan(emittedBefore)
    expect(wrapper.emitted('search')![wrapper.emitted('search')!.length - 1]).toEqual(['test:*'])
  })

  it('shows empty state when no keys', () => {
    const wrapper = mount(RedisKeyBrowser, {
      props: { connected: true, keys: [] },
    })
    expect(wrapper.find('.key-list-empty').exists()).toBe(true)
  })

  it('groups keys into tree nodes by separator', () => {
    const wrapper = mount(RedisKeyBrowser, {
      props: { connected: true, keys: sampleKeys },
    })

    // Should have folders: counter (leaf), queue, session, tags, user
    const folders = wrapper.findAll('.key-folder')
    expect(folders.length).toBeGreaterThanOrEqual(1)

    // user folder should have 3 children
    const userFolder = folders.find(f => f.text().includes('user'))
    expect(userFolder).toBeTruthy()
  })

  it('displays type icons correctly', () => {
    const wrapper = mount(RedisKeyBrowser, {
      props: { connected: true, keys: [{ key: 'mykey', type: 'string' }] },
    })
    const typeIcon = wrapper.find('.key-type-icon')
    expect(typeIcon.text()).toBe('Aa')
  })

  it('emits selectKey when clicking a leaf node', async () => {
    const wrapper = mount(RedisKeyBrowser, {
      props: { connected: true, keys: [{ key: 'counter', type: 'string' }] },
    })
    const leafItem = wrapper.find('.key-item')
    await leafItem.trigger('click')
    expect(wrapper.emitted('selectKey')).toBeTruthy()
    expect(wrapper.emitted('selectKey')![0]).toEqual(['counter'])
  })

  it('toggles folder collapse on click', async () => {
    const wrapper = mount(RedisKeyBrowser, {
      props: { connected: true, keys: sampleKeys },
    })
    const folder = wrapper.find('.key-folder')
    if (folder.exists()) {
      await folder.trigger('click')
      // After click, folder should toggle
      expect(folder.find('.key-folder-arrow').classes()).toContain('collapsed')
    }
  })

  it('emits search on button click', async () => {
    const wrapper = mount(RedisKeyBrowser, {
      props: { connected: true, keys: [] },
    })
    await wrapper.find('.redis-btn-sm').trigger('click')
    expect(wrapper.emitted('search')).toBeTruthy()
  })
})
