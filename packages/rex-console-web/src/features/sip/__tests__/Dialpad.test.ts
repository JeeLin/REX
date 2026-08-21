import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import Dialpad from '../Dialpad.vue'

// i18n stub: returns the key itself.
vi.mock('vue-i18n', () => ({ useI18n: () => ({ t: (k: string) => k }) }))

describe('Dialpad', () => {
  it('emits dial with the typed number when registered', async () => {
    const w = mount(Dialpad, { props: { registered: true } })
    await w.find('.dial-number').setValue('1000')
    await w.find('.dialpad-btn-call').trigger('click')
    expect(w.emitted('dial')?.[0]).toEqual(['1000'])
  })

  it('does not emit dial when not registered', async () => {
    const w = mount(Dialpad, { props: { registered: false } })
    await w.find('.dial-number').setValue('1000')
    await w.find('.dialpad-btn-call').trigger('click')
    expect(w.emitted('dial')).toBeUndefined()
  })

  it('builds number by pressing keys', async () => {
    const w = mount(Dialpad, { props: { registered: true } })
    for (const k of ['1', '2', '3']) {
      await w.findAll('.dialpad-key').find((b) => b.text() === k)!.trigger('click')
    }
    expect((w.find('.dial-number').element as HTMLInputElement).value).toBe('123')
  })
})
