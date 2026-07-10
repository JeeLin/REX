import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import BatchOperationDialog from '../BatchOperationDialog.vue'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}))

describe('BatchOperationDialog', () => {
  const mockKeys = ['key1', 'key2', 'key3']

  const createWrapper = (props = {}) => {
    return mount(BatchOperationDialog, {
      props: {
        visible: true,
        keys: mockKeys,
        operation: 'delete',
        ...props,
      },
      global: {
        stubs: {
          Teleport: true,
        },
      },
    })
  }

  it('renders delete confirmation message', () => {
    const wrapper = createWrapper({ operation: 'delete' })
    expect(wrapper.text()).toContain('redis.keys.batch.deleteConfirm')
  })

  it('renders TTL input when operation is setTtl', () => {
    const wrapper = createWrapper({ operation: 'setTtl' })
    expect(wrapper.find('input[type="number"]').exists()).toBe(true)
  })

  it('renders export format options when operation is export', () => {
    const wrapper = createWrapper({ operation: 'export' })
    expect(wrapper.find('input[type="radio"]').exists()).toBe(true)
  })

  it('emits close event when close button clicked', async () => {
    const wrapper = createWrapper()
    await wrapper.find('.batch-dialog-close').trigger('click')
    expect(wrapper.emitted('close')).toBeTruthy()
  })

  it('emits confirmDelete event with keys', async () => {
    const wrapper = createWrapper({ operation: 'delete' })
    await wrapper.find('.batch-dialog-btn-danger').trigger('click')
    expect(wrapper.emitted('confirmDelete')).toBeTruthy()
    expect(wrapper.emitted('confirmDelete')![0]).toEqual([mockKeys])
  })
})