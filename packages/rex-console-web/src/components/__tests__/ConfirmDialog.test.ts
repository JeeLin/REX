import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import ConfirmDialog from '../ConfirmDialog.vue'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}))

vi.mock('@/composables/useId', () => ({
  useId: (prefix: string) => `${prefix}-test-id`,
}))

describe('ConfirmDialog', () => {
  const defaultProps = {
    visible: true,
    title: 'Confirm Action',
    message: 'Are you sure you want to proceed?',
  }

  function mountDialog(props = defaultProps) {
    return mount(ConfirmDialog, {
      props,
      global: {
        stubs: { Teleport: true },
      },
    })
  }

  it('renders title and message when visible', () => {
    const wrapper = mountDialog()
    expect(wrapper.find('.confirm-title').text()).toBe('Confirm Action')
    expect(wrapper.find('.confirm-message').text()).toBe('Are you sure you want to proceed?')
  })

  it('does not render when not visible', () => {
    const wrapper = mountDialog({ ...defaultProps, visible: false })
    expect(wrapper.find('.confirm-dialog').exists()).toBe(false)
  })

  it('emits confirm when confirm button is clicked', async () => {
    const wrapper = mountDialog()
    await wrapper.find('.confirm-actions button:last-child').trigger('click')
    expect(wrapper.emitted('confirm')).toHaveLength(1)
  })

  it('emits cancel when cancel button is clicked', async () => {
    const wrapper = mountDialog()
    await wrapper.find('.confirm-actions button:first-child').trigger('click')
    expect(wrapper.emitted('cancel')).toHaveLength(1)
  })

  it('emits cancel when overlay is clicked', async () => {
    const wrapper = mountDialog()
    await wrapper.find('.confirm-overlay').trigger('click')
    expect(wrapper.emitted('cancel')).toHaveLength(1)
  })

  it('uses default labels from i18n', () => {
    const wrapper = mountDialog()
    const buttons = wrapper.findAll('.confirm-actions button')
    expect(buttons).toHaveLength(2)
    expect(buttons[0]!.text()).toBeTruthy()
    expect(buttons[1]!.text()).toBeTruthy()
  })

  it('has correct aria attributes', () => {
    const wrapper = mountDialog()
    const dialog = wrapper.find('.confirm-dialog')
    expect(dialog.attributes('role')).toBe('alertdialog')
    expect(dialog.attributes('aria-modal')).toBe('true')
  })
})
