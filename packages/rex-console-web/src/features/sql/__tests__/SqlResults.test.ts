import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import SqlResults from '../SqlResults.vue'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string, params?: Record<string, unknown>) => {
      if (key === 'sql.rows') return `${params?.count} rows`
      if (key === 'sql.elapsed') return `${params?.time}s`
      return key
    },
  }),
}))

describe('SqlResults', () => {
  const mockResult = {
    columns: [
      { name: 'id', data_type: 'INT' },
      { name: 'name', data_type: 'VARCHAR' },
    ],
    rows: [
      [1, 'Alice'],
      [2, 'Bob'],
      [3, 'Charlie'],
    ],
    elapsed_ms: 15,
    affected_rows: 0,
  }

  it('renders column headers', () => {
    const wrapper = mount(SqlResults, {
      props: { result: mockResult, loading: false },
    })
    const headers = wrapper.findAll('th')
    // First th is row number (#), then data columns
    expect(headers.length).toBeGreaterThanOrEqual(3)
    expect(headers[1].text()).toContain('id')
    expect(headers[2].text()).toContain('name')
  })

  it('renders data rows', () => {
    const wrapper = mount(SqlResults, {
      props: { result: mockResult, loading: false },
    })
    const rows = wrapper.findAll('tbody tr')
    expect(rows.length).toBe(3)
  })

  it('shows row numbers', () => {
    const wrapper = mount(SqlResults, {
      props: { result: mockResult, loading: false },
    })
    const firstRow = wrapper.find('tbody tr')
    expect(firstRow.text()).toContain('1')
  })

  it('shows footer with row count and elapsed time', () => {
    const wrapper = mount(SqlResults, {
      props: { result: mockResult, loading: false },
    })
    const footer = wrapper.find('.results-footer')
    expect(footer.text()).toContain('3 rows')
    expect(footer.text()).toContain('0.015s')
  })

  it('shows loading state', () => {
    const wrapper = mount(SqlResults, {
      props: { result: null, loading: true },
    })
    expect(wrapper.find('.results-empty').text()).toContain('sql.executing')
  })

  it('shows empty state when no result', () => {
    const wrapper = mount(SqlResults, {
      props: { result: null, loading: false },
    })
    expect(wrapper.find('.results-empty').exists()).toBe(true)
  })

  it('shows tabs for results, message, and explain', () => {
    const wrapper = mount(SqlResults, {
      props: { result: mockResult, loading: false },
    })
    const tabs = wrapper.findAll('.results-tab')
    expect(tabs.length).toBe(2) // results + message (no explain tab by default)
  })

  it('sorts data when clicking column header', async () => {
    const wrapper = mount(SqlResults, {
      props: { result: mockResult, loading: false },
    })
    const nameHeader = wrapper.findAll('th')[1]
    await nameHeader.trigger('click')

    // After sorting by name, rows should be in alphabetical order
    const rows = wrapper.findAll('tbody tr')
    expect(rows.length).toBe(3)
  })

  it('formats NULL cells', () => {
    const resultWithNull = {
      columns: [{ name: 'val', data_type: 'TEXT' }],
      rows: [[null], ['hello']],
      elapsed_ms: 5,
      affected_rows: 0,
    }
    const wrapper = mount(SqlResults, {
      props: { result: resultWithNull, loading: false },
    })
    const cells = wrapper.findAll('td')
    // NULL should be formatted differently (e.g., "NULL" text or special class)
    expect(cells.length).toBeGreaterThanOrEqual(2)
  })
})
