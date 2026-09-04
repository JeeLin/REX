import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import FormatViewer from '../FormatViewer.vue'

describe('FormatViewer', () => {
  it('renders text content by default', () => {
    const w = mount(FormatViewer, { props: { value: 'hello world' } })
    expect(w.find('.format-content').text()).toBe('hello world')
  })

  it('detects JSON format and auto-selects the JSON tab', async () => {
    const json = JSON.stringify({ key: 'value' })
    const w = mount(FormatViewer, { props: { value: json } })
    // displayValue should be pretty-printed
    expect(w.find('.format-content').text()).toContain('"key"')
    expect(w.find('.format-content').text()).toContain('value')
    // JSON tab should be active
    const jsonTab = w.findAll('.format-tab').find(b => b.text().includes('JSON'))!
    expect(jsonTab.classes()).toContain('format-tab--active')
  })

  it('switches format when a tab is clicked', async () => {
    const w = mount(FormatViewer, { props: { value: 'AB' } })
    // Default is text
    expect(w.find('.format-content').text()).toBe('AB')

    // Click Hex tab
    const hexTab = w.findAll('.format-tab').find(b => b.text().includes('Hex'))!
    await hexTab.trigger('click')
    expect(w.findAll('.format-tab').find(b => b.text().includes('Hex'))!.classes()).toContain('format-tab--active')

    // 'AB' → bytes 0x41 0x42
    expect(w.find('.format-content').text()).toBe('41 42')
  })

  it('shows binary encoding with 8-bit padded strings', async () => {
    const w = mount(FormatViewer, { props: { value: 'A' } })
    const binTab = w.findAll('.format-tab').find(b => b.text().includes('Binary'))!
    await binTab.trigger('click')
    // 'A' = 0x41 = 01000001
    expect(w.find('.format-content').text()).toBe('01000001')
  })

  it('uses backend decoded content for advanced (non-basic) formats', async () => {
    const w = mount(FormatViewer, {
      props: {
        value: '\\x00\\x01',
        formatInfo: { detected: 'msgpack', decoded: '{ "foo": 1 }' },
      },
    })
    // The msgpack tab should appear as a 5th option
    const tabs = w.findAll('.format-tab')
    const labels = tabs.map(t => t.text())
    expect(labels).toContain('Msgpack')
    // Should be active (detected → active)
    expect(w.find('.format-content').text()).toBe('{ "foo": 1 }')
  })
})
