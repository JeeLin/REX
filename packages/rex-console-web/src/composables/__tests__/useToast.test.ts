import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'

describe('useToast', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    vi.resetModules()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  async function getToast() {
    const { useToast } = await import('../useToast')
    return useToast()
  }

  it('starts with empty toasts', async () => {
    const { toasts } = await getToast()
    expect(toasts.value).toEqual([])
  })

  it('adds success toast', async () => {
    const { success, toasts } = await getToast()
    success('Done!')
    expect(toasts.value.length).toBe(1)
    expect(toasts.value[0]!.type).toBe('success')
    expect(toasts.value[0]!.message).toBe('Done!')
  })

  it('adds error toast with 5s duration', async () => {
    const { error, toasts } = await getToast()
    error('Oops')
    expect(toasts.value.length).toBe(1)
    expect(toasts.value[0]!.type).toBe('error')

    vi.advanceTimersByTime(4999)
    expect(toasts.value.length).toBe(1)

    vi.advanceTimersByTime(1)
    expect(toasts.value.length).toBe(0)
  })

  it('adds warning toast with 4s duration', async () => {
    const { warning, toasts } = await getToast()
    warning('Careful')
    expect(toasts.value[0]!.type).toBe('warning')

    vi.advanceTimersByTime(3999)
    expect(toasts.value.length).toBe(1)

    vi.advanceTimersByTime(1)
    expect(toasts.value.length).toBe(0)
  })

  it('adds info toast with 3s duration', async () => {
    const { info, toasts } = await getToast()
    info('FYI')
    expect(toasts.value[0]!.type).toBe('info')

    vi.advanceTimersByTime(2999)
    expect(toasts.value.length).toBe(1)

    vi.advanceTimersByTime(1)
    expect(toasts.value.length).toBe(0)
  })

  it('removes toast by id', async () => {
    const { success, remove, toasts } = await getToast()
    success('Test')
    const id = toasts.value[0]!.id
    remove(id)
    expect(toasts.value.length).toBe(0)
  })

  it('removes only the specified toast', async () => {
    const { success, remove, toasts } = await getToast()
    success('First')
    success('Second')
    expect(toasts.value.length).toBe(2)

    const id = toasts.value[0]!.id
    remove(id)
    expect(toasts.value.length).toBe(1)
    expect(toasts.value[0]!.message).toBe('Second')
  })

  it('generates unique ids', async () => {
    const { success, toasts } = await getToast()
    success('One')
    success('Two')
    expect(toasts.value[0]!.id).not.toBe(toasts.value[1]!.id)
  })
})
