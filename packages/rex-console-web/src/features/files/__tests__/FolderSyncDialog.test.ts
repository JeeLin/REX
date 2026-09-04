import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import FolderSyncDialog from '../FolderSyncDialog.vue'

vi.mock('vue-i18n', () => ({ useI18n: () => ({ t: (k: string) => k }) }))

vi.mock('@/components/ui/Button.vue', () => ({
  default: {
    template: '<button @click="$emit(\'click\', $event)"><slot /></button>',
    props: ['variant', 'icon', 'size', 'disabled'],
  },
}))

beforeEach(() => {
  vi.clearAllMocks()
})

const defaultProps = {
  visible: true,
  sourcePath: '/home/user/project/',
  targetPath: '/var/www/html/',
}

// Stub Teleport so its children render inline in the wrapper DOM
const mountOpts = { global: { stubs: { Teleport: true } } }

describe('FolderSyncDialog', () => {
  it('renders dialog content when visible is true', () => {
    const wrapper = mount(FolderSyncDialog, { ...mountOpts, props: defaultProps })

    expect(wrapper.find('.fsd-dialog').exists()).toBe(true)
    expect(wrapper.text()).toContain('files.folderSync')
  })

  it('does not render dialog when visible is false', () => {
    const wrapper = mount(FolderSyncDialog, {
      ...mountOpts,
      props: { ...defaultProps, visible: false },
    })

    expect(wrapper.find('.fsd-dialog').exists()).toBe(false)
  })

  it('shows source and target paths', () => {
    const wrapper = mount(FolderSyncDialog, { ...mountOpts, props: defaultProps })

    expect(wrapper.text()).toContain('/home/user/project/')
    expect(wrapper.text()).toContain('/var/www/html/')
  })

  it('has direction selector with upload/download/bidirectional options', () => {
    const wrapper = mount(FolderSyncDialog, { ...mountOpts, props: defaultProps })

    const radios = wrapper.findAll('input[type="radio"]')
    expect(radios.length).toBe(3)

    const values = radios.map(r => (r.element as HTMLInputElement).value)
    expect(values).toEqual(['upload', 'download', 'bidirectional'])
  })

  it('emits close when close button is clicked', async () => {
    const wrapper = mount(FolderSyncDialog, { ...mountOpts, props: defaultProps })

    // Click the × close button in the header
    await wrapper.find('.fsd-close').trigger('click')
    expect(wrapper.emitted('close')).toHaveLength(1)
  })
})
