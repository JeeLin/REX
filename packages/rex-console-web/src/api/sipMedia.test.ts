import { describe, it, expect, vi } from 'vitest'
import {
  encodeVideoFrame,
  decodeVideoFrame,
  paintRgbaToCanvas,
  SipVideo,
  KIND_VIDEO,
  VIDEO_PIXFMT_RGBA,
} from './sipMedia'

// 2x2 RGBA 像素：红/绿/蓝/白 行优先。
const RGBA = new Uint8Array([
  255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
])

describe('videoFrame encode/decode', () => {
  it('roundtrip preserves width/height/pixels', () => {
    const buf = encodeVideoFrame(2, 2, RGBA)
    const bytes = new Uint8Array(buf)
    expect(bytes[0]).toBe(VIDEO_PIXFMT_RGBA)
    expect(bytes[1]).toBe(2) // w LE
    expect(bytes[3]).toBe(2) // h LE
    const { width, height, rgba } = decodeVideoFrame(buf)
    expect(width).toBe(2)
    expect(height).toBe(2)
    expect(rgba).toEqual(RGBA)
  })

  it('encode rejects size mismatch', () => {
    expect(() => encodeVideoFrame(2, 2, RGBA.subarray(0, 8))).toThrow()
  })

  it('decode rejects unknown fmt', () => {
    const bad = new Uint8Array([99, 1, 0, 1, 0, 1, 2, 3, 4])
    const buf = bad.buffer.slice(bad.byteOffset, bad.byteOffset + bad.byteLength)
    expect(() => decodeVideoFrame(buf as ArrayBuffer)).toThrow()
  })

  it('decode rejects too-short frame', () => {
    expect(() => decodeVideoFrame(new Uint8Array([0, 1]).buffer)).toThrow()
  })
})

describe('KIND_VIDEO constant', () => {
  it('equals backend kind=2', () => {
    expect(KIND_VIDEO).toBe(2)
  })
})

describe('SipVideo render', () => {
  it('paints RGBA into a mock 2D canvas context', () => {
    const put = vi.fn()
    const ctx = { putImageData: put } as unknown as CanvasRenderingContext2D
    paintRgbaToCanvas(ctx, 2, 2, RGBA)
    expect(put).toHaveBeenCalledOnce()
    const arg = put.mock.calls[0]![0] as ImageData
    expect(arg.width).toBe(2)
    expect(arg.height).toBe(2)
    expect(Array.from(arg.data)).toEqual(Array.from(RGBA))
  })

  it('renderFrame resizes attached canvas to frame size', () => {
    const canvas = {
      width: 0,
      height: 0,
      getContext: () => ({ putImageData: () => {} }),
    } as unknown as HTMLCanvasElement
    const video = new SipVideo({ createCanvas: () => document.createElement('canvas') })
    video.attachCanvas(canvas)
    video.renderFrame(4, 4, new Uint8Array(4 * 4 * 4))
    expect(canvas.width).toBe(4)
    expect(canvas.height).toBe(4)
  })

  it('supported is false when no canvas API (jsdom-lite)', () => {
    const docSpy = vi.spyOn(globalThis, 'document', 'get')
    docSpy.mockReturnValue(undefined as unknown as Document)
    try {
      const video = new SipVideo({ document: undefined })
      expect(video.supported).toBe(false)
    } finally {
      docSpy.mockRestore()
    }
  })
})
