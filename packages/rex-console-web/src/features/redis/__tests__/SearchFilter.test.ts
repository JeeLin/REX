import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import SearchFilter from '../SearchFilter.vue'

// Mock vue-i18n
vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}))

describe('SearchFilter', () => {
  const createWrapper = (props = {}) => {
    return mount(SearchFilter, { props })
  }

  it('renders pattern input and search button', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('.filter-pattern').exists()).toBe(true)
    expect(wrapper.find('.redis-btn-sm').exists()).toBe(true)
  })

  it('renders type filter dropdown', () => {
    const wrapper = createWrapper()
    const select = wrapper.find('.filter-select')
    expect(select.exists()).toBe(true)
    const options = select.findAll('option')
    expect(options.length).toBe(7) // all types + 6 individual types
  })

  it('renders TTL range inputs', () => {
    const wrapper = createWrapper()
    const ttlInputs = wrapper.findAll('.filter-ttl-input')
    expect(ttlInputs.length).toBe(2)
  })

  it('emits search event with pattern on button click', async () => {
    const wrapper = createWrapper()
    const input = wrapper.find('.filter-pattern')
    await input.setValue('user:*')
    await wrapper.find('.redis-btn-sm').trigger('click')
    expect(wrapper.emitted('search')).toBeTruthy()
    expect(wrapper.emitted('search')![0]).toEqual(['user:*'])
  })

  it('emits search event on Enter key', async () => {
    const wrapper = createWrapper()
    const input = wrapper.find('.filter-pattern')
    await input.setValue('user:*')
    await input.trigger('keydown.enter')
    expect(wrapper.emitted('search')![0]).toEqual(['user:*'])
  })

  it('emits filter event when type changes', async () => {
    const wrapper = createWrapper()
    const select = wrapper.find('.filter-select')
    await select.setValue('string')
    expect(wrapper.emitted('filter')).toBeTruthy()
    const lastFilter = wrapper.emitted('filter')!.slice(-1)[0]
    expect(lastFilter).toBeDefined()
    expect(lastFilter![0]).toEqual({
      type: 'string',
      ttlMin: null,
      ttlMax: null,
    })
  })

  it('emits filter event with TTL range', async () => {
    const wrapper = createWrapper()
    const ttlInputs = wrapper.findAll('.filter-ttl-input')
    expect(ttlInputs.length).toBeGreaterThanOrEqual(2)
    await ttlInputs[0]!.setValue(60)
    await ttlInputs[1]!.setValue(3600)
    await ttlInputs[0]!.trigger('keydown.enter')
    const lastFilter = wrapper.emitted('filter')!.slice(-1)[0]
    expect(lastFilter).toBeDefined()
    expect(lastFilter![0]).toEqual({
      type: '',
      ttlMin: 60,
      ttlMax: 3600,
    })
  })

  it('shows active filter badges when type is selected', async () => {
    const wrapper = createWrapper()
    const select = wrapper.find('.filter-select')
    await select.setValue('hash')
    expect(wrapper.find('.filter-badge').exists()).toBe(true)
    expect(wrapper.find('.filter-badge').text()).toContain('hash')
  })

  it('clears all filters on clear all button', async () => {
    const wrapper = createWrapper()
    // Set a type filter
    const select = wrapper.find('.filter-select')
    await select.setValue('hash')
    expect(wrapper.find('.filter-badge').exists()).toBe(true)
    // Clear all
    await wrapper.find('.filter-clear-all').trigger('click')
    const lastFilter = wrapper.emitted('filter')!.slice(-1)[0]
    expect(lastFilter).toBeDefined()
    expect(lastFilter![0]).toEqual({
      type: '',
      ttlMin: null,
      ttlMax: null,
    })
  })

  it('defaults pattern to *', () => {
    const wrapper = createWrapper()
    expect((wrapper.find('.filter-pattern').element as HTMLInputElement).value).toBe('*')
  })
})
