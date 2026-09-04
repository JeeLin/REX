import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import CommandPalette from '../CommandPalette.vue'

vi.mock('vue-i18n', () => ({ useI18n: () => ({ t: (k: string) => k, locale: { value: 'en' } }) }))
vi.mock('vue-router', () => ({ useRouter: () => ({ push: vi.fn() }) }))
vi.mock('@/stores/environments', () => ({
  useEnvironmentsStore: () => ({
    environments: [
      { id: 'env-1', name: 'Production' },
      { id: 'env-2', name: 'Staging' },
    ],
    fetchEnvironments: vi.fn(),
  }),
}))

describe('CommandPalette', () => {
  let wrapper: ReturnType<typeof mount>

  function mountPalette(visible = true) {
    wrapper = mount(CommandPalette, {
      props: { visible },
      global: {
        stubs: { teleport: true },
      },
    })
    return wrapper
  }

  beforeEach(() => {
    wrapper?.unmount()
  })

  it('does not render the overlay when visible is false', () => {
    mountPalette(false)
    expect(wrapper.find('.palette-overlay').exists()).toBe(false)
  })

  it('renders the palette overlay and input when visible', () => {
    mountPalette(true)
    expect(wrapper.find('.palette-overlay').exists()).toBe(true)
    expect(wrapper.find('.palette-input').exists()).toBe(true)
  })

  it('filters commands by search query', async () => {
    mountPalette(true)
    const input = wrapper.find('.palette-input')
    await input.setValue('Settings')
    // After filtering, only the "Settings" command (and possibly settings category) should remain.
    const items = wrapper.findAll('.palette-item-label')
    const labels = items.map((el) => el.text())
    expect(labels.length).toBeGreaterThan(0)
    // At least one item should contain "Settings" in its label (case-insensitive via i18n key passthrough).
    expect(labels.some((l) => l.toLowerCase().includes('settings'))).toBe(true)
  })

  it('shows "no results" when search matches nothing', async () => {
    mountPalette(true)
    await wrapper.find('.palette-input').setValue('zzz-nonexistent-query-zzz')
    expect(wrapper.find('.palette-empty').exists()).toBe(true)
  })

  it('emits close when Escape is pressed', async () => {
    mountPalette(true)
    await document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    // The palette attaches the listener on mount; Escape triggers emit('close').
    expect(wrapper.emitted('close')).toBeTruthy()
  })
})
