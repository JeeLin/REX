import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import SkeletonLoader from '../SkeletonLoader.vue'

describe('SkeletonLoader', () => {
  it('renders card variant with stat cards and card grid', () => {
    const wrapper = mount(SkeletonLoader, { props: { variant: 'card' } })
    expect(wrapper.find('.skeleton-wrapper').exists()).toBe(true)
    expect(wrapper.find('.skeleton-stats-row').exists()).toBe(true)
    expect(wrapper.findAll('.skeleton-stat-card').length).toBe(4)
    expect(wrapper.find('.skeleton-cards-grid').exists()).toBe(true)
  })

  it('renders card variant with custom count', () => {
    const wrapper = mount(SkeletonLoader, { props: { variant: 'card', count: 5 } })
    const grid = wrapper.find('.skeleton-cards-grid')
    expect(grid.findAll('.skeleton-card').length).toBe(5)
  })

  it('renders list variant with default 4 cards', () => {
    const wrapper = mount(SkeletonLoader, { props: { variant: 'list' } })
    expect(wrapper.find('.skeleton-cards-grid').exists()).toBe(true)
    expect(wrapper.findAll('.skeleton-card').length).toBe(4)
  })

  it('renders list variant with custom count', () => {
    const wrapper = mount(SkeletonLoader, { props: { variant: 'list', count: 2 } })
    expect(wrapper.findAll('.skeleton-card').length).toBe(2)
  })

  it('renders table variant with default 5 rows', () => {
    const wrapper = mount(SkeletonLoader, { props: { variant: 'table' } })
    expect(wrapper.find('.skeleton-table').exists()).toBe(true)
    expect(wrapper.findAll('.skeleton-table-row').length).toBe(5)
  })

  it('renders table variant with custom count', () => {
    const wrapper = mount(SkeletonLoader, { props: { variant: 'table', count: 8 } })
    expect(wrapper.findAll('.skeleton-table-row').length).toBe(8)
  })

  it('applies shimmer animation class on skeleton lines', () => {
    const wrapper = mount(SkeletonLoader, { props: { variant: 'card' } })
    const lines = wrapper.findAll('.skeleton-line')
    expect(lines.length).toBeGreaterThan(0)
  })
})
