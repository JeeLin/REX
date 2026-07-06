import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import MobileToolbar from '../MobileToolbar.vue'

describe('MobileToolbar', () => {
  it('renders when visible is true', () => {
    const wrapper = mount(MobileToolbar, {
      props: { visible: true },
      slots: {
        default: '<button class="test-btn">Click</button>',
      },
    })
    expect(wrapper.find('.mobile-toolbar').exists()).toBe(true)
  })

  it('does not render when visible is false', () => {
    const wrapper = mount(MobileToolbar, {
      props: { visible: false },
      slots: {
        default: '<button class="test-btn">Click</button>',
      },
    })
    expect(wrapper.find('.mobile-toolbar').exists()).toBe(false)
  })

  it('renders slot content', () => {
    const wrapper = mount(MobileToolbar, {
      props: { visible: true },
      slots: {
        default: '<button class="test-btn">Click</button>',
      },
    })
    expect(wrapper.find('.test-btn').exists()).toBe(true)
    expect(wrapper.find('.test-btn').text()).toBe('Click')
  })

  it('renders multiple slot children', () => {
    const wrapper = mount(MobileToolbar, {
      props: { visible: true },
      slots: {
        default: '<div class="row1">Row 1</div><div class="row2">Row 2</div>',
      },
    })
    expect(wrapper.find('.row1').exists()).toBe(true)
    expect(wrapper.find('.row2').exists()).toBe(true)
  })
})
