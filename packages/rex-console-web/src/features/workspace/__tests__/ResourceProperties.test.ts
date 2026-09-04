import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import ResourceProperties from '../ResourceProperties.vue'

vi.mock('vue-i18n', () => ({ useI18n: () => ({ t: (k: string) => k }) }))

// Stub UI components to keep the test focused on the dialog logic.
vi.mock('@/components/ui/Modal.vue', () => ({
  default: {
    template: `
      <div class="modal-stub" v-if="modelValue">
        <slot name="default" />
        <slot name="footer" />
      </div>`,
    props: ['modelValue', 'title', 'width'],
    emits: ['update:modelValue'],
  },
}))
vi.mock('@/components/ui/Tabs.vue', () => ({
  default: {
    template: '<div class="tabs-stub"><slot name="item" :tab="tabs[0]" /><slot /></div>',
    props: ['modelValue', 'tabs'],
    emits: ['update:modelValue'],
  },
}))
vi.mock('@/components/ui/Input.vue', () => ({
  default: {
    template: '<input class="input-stub" :value="modelValue" @input="$emit(\'update:modelValue\', $event.target.value)" />',
    props: ['modelValue', 'size', 'placeholder'],
    emits: ['update:modelValue'],
  },
}))
vi.mock('@/components/ui/Select.vue', () => ({
  default: {
    template: '<select class="select-stub" />',
    props: ['modelValue', 'options', 'size'],
    emits: ['update:modelValue'],
  },
}))
vi.mock('@/components/ui/Button.vue', () => ({
  default: {
    template: '<button class="btn-stub" @click="$emit(\'click\')"><slot /></button>',
    props: ['variant'],
    emits: ['click'],
  },
}))

describe('ResourceProperties', () => {
  function mountDialog(show = true, resource?: { name?: string; protocol?: string }) {
    return mount(ResourceProperties, {
      props: {
        show,
        resource: resource ?? { name: 'Prod Server', protocol: 'ssh' },
      },
      global: { stubs: { teleport: true } },
    })
  }

  it('does not render the modal when show is false', () => {
    const w = mountDialog(false)
    expect(w.find('.modal-stub').exists()).toBe(false)
  })

  it('renders the modal when show is true', () => {
    const w = mountDialog(true)
    expect(w.find('.modal-stub').exists()).toBe(true)
  })

  it('displays the resource name in the form', async () => {
    const w = mountDialog(true, { name: 'Dev Redis', protocol: 'redis' })
    await w.vm.$nextTick()
    // The Modal title receives "Properties — {name}".
    const modal = w.findComponent({ name: '' }) // fallback: check the stub prop
    // More robust: the component sets form.name from the prop.
    // Verify the name input has the correct value.
    const nameInput = w.findAll('.input-stub').at(0)
    expect(nameInput!.element.value).toBe('Dev Redis')
  })

  it('emits update:show with false when Cancel is clicked', async () => {
    const w = mountDialog(true)
    const cancelBtn = w.findAll('.btn-stub').find((b) => b.text() === 'Cancel')
    await cancelBtn!.trigger('click')
    expect(w.emitted('update:show')).toBeTruthy()
    expect(w.emitted('update:show')![0]).toEqual([false])
  })

  it('emits save with form data when Save is clicked', async () => {
    const w = mountDialog(true, { name: 'My Server', protocol: 'ssh' })
    const saveBtn = w.findAll('.btn-stub').find((b) => b.text() === 'Save')
    await saveBtn!.trigger('click')
    expect(w.emitted('save')).toBeTruthy()
    const emittedData = w.emitted('save')![0]![0] as { name: string; protocol: string }
    expect(emittedData.name).toBe('My Server')
    expect(emittedData.protocol).toBe('ssh')
  })
})
