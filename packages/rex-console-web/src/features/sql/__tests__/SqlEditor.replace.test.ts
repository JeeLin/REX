import { describe, it, expect, afterEach } from 'vitest'
import { mount, type VueWrapper } from '@vue/test-utils'
import SqlEditor from '../SqlEditor.vue'

let wrapper: VueWrapper | undefined

function press(target: Element, key: string, init: KeyboardEventInit = {}) {
  target.dispatchEvent(new KeyboardEvent('keydown', {
    key,
    code: `Key${key.toUpperCase()}`,
    keyCode: key.toUpperCase().charCodeAt(0),
    bubbles: true,
    cancelable: true,
    ...init,
  }))
}

async function mountEditor() {
  wrapper = mount(SqlEditor, { props: { modelValue: 'select 1' }, attachTo: document.body })
  await new Promise(resolve => setTimeout(resolve, 0))
  const content = wrapper.element.querySelector('.cm-content')
  expect(content).toBeTruthy()
  return { wrapper, content: content! }
}

afterEach(() => {
  wrapper?.unmount()
  wrapper = undefined
  document.body.innerHTML = ''
})

describe('SqlEditor Ctrl+Shift+R', () => {
  it('opens the search panel with the replace field focused', async () => {
    const { wrapper: w, content } = await mountEditor()

    press(content, 'R', { ctrlKey: true, shiftKey: true })

    const panel = w.element.querySelector('.cm-search')
    expect(panel).toBeTruthy()

    const replaceField = panel!.querySelector('input[name="replace"]') as HTMLInputElement
    expect(replaceField).toBeTruthy()
    expect(document.activeElement).toBe(replaceField)
  })

  it('does not open the panel on plain Ctrl+R', async () => {
    const { wrapper: w, content } = await mountEditor()

    press(content, 'r', { ctrlKey: true })

    expect(w.element.querySelector('.cm-search')).toBeNull()
  })
})
