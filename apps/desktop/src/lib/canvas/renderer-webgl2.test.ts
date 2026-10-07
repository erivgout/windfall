import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { rgba } from "./color"
import { RectBatch } from "./rect-batch"
import { RendererUnavailableError, type RectRenderer } from "./renderer"
import {
  CONTEXT_RESTORE_WAIT_MS,
  createWebGL2Renderer,
} from "./renderer-webgl2"
import { createFakeGl, type FakeGl } from "./test-utils"
import { deriveGridTheme } from "./theme"
import { deviceTransform } from "./viewport"

const theme = deriveGridTheme({
  background: rgba(20, 20, 20),
  foreground: rgba(250, 250, 250),
  mutedForeground: rgba(160, 160, 160),
  brand: rgba(230, 60, 140),
  playhead: rgba(90, 160, 255),
  gridLine: rgba(255, 255, 255, 18),
  gridLineStrong: rgba(255, 255, 255, 51),
})

const transform = deviceTransform({
  width: 800,
  height: 400,
  dpr: 1,
  scrollTick: 0,
  scrollRow: 0,
  pxPerTick: 0.0625,
  rowHeight: 16,
})

function setUp(gl: FakeGl = createFakeGl()) {
  const canvas = document.createElement("canvas")
  Object.defineProperty(canvas, "getContext", { value: () => gl.context })
  const renderer = createWebGL2Renderer(canvas)
  renderer.setTheme(theme)
  const batch = new RectBatch()
  batch.push(1, 960, 960, 3, 1, rgba(200, 100, 50))
  const lose = () => {
    gl.lose()
    const event = new Event("webglcontextlost", { cancelable: true })
    canvas.dispatchEvent(event)
    return event
  }
  const restore = () => {
    gl.restore()
    canvas.dispatchEvent(new Event("webglcontextrestored"))
  }
  return { canvas, gl, renderer, batch, lose, restore }
}

function drawFrame(renderer: RectRenderer, batch: RectBatch): void {
  renderer.beginFrame(transform)
  renderer.drawBatch(batch)
  renderer.endFrame()
}

describe("WebGL2 renderer", () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })
  afterEach(() => {
    vi.useRealTimers()
  })

  it("draws a batch with one instanced call and uploads it once", () => {
    const { gl, renderer, batch } = setUp()
    drawFrame(renderer, batch)
    drawFrame(renderer, batch)
    expect(gl.count("drawArraysInstanced")).toBe(2)
    expect(gl.count("createVertexArray")).toBe(1)
    expect(gl.count("bufferData")).toBe(3)
  })

  it("keeps the browser from giving up on a lost context", () => {
    const { lose } = setUp()
    // Without preventDefault the context is never restored.
    expect(lose().defaultPrevented).toBe(true)
  })

  it("draws nothing while the context is lost", () => {
    const { gl, renderer, batch, lose } = setUp()
    lose()
    const before = gl.calls.length
    drawFrame(renderer, batch)
    expect(gl.calls.slice(before)).toEqual([])
    expect(renderer.finish()).toBe(false)
    expect(renderer.takeGpuTimes()).toEqual([])
  })

  it("skips a frame when the context is lost before the event says so", () => {
    const { gl, renderer, batch } = setUp()
    gl.lose()
    expect(() => drawFrame(renderer, batch)).not.toThrow()
    expect(gl.count("drawArraysInstanced")).toBe(0)
    expect(gl.count("createVertexArray")).toBe(0)
  })

  it("builds its program again when the context is restored, and asks for a redraw", () => {
    const { gl, renderer, batch, lose, restore } = setUp()
    const restored = vi.fn()
    const gone = vi.fn()
    renderer.onRestored = restored
    renderer.onLost = gone
    drawFrame(renderer, batch)
    expect(gl.count("createProgram")).toBe(1)

    lose()
    restore()
    expect(restored).toHaveBeenCalledTimes(1)
    expect(gl.count("createProgram")).toBe(2)
    expect(gl.count("enable")).toBe(2)

    // The buffers went with the old context, so the batch is uploaded
    // again although it has not changed.
    drawFrame(renderer, batch)
    expect(gl.count("createVertexArray")).toBe(2)
    expect(gl.count("bufferData")).toBe(6)
    expect(gl.count("drawArraysInstanced")).toBe(2)

    // It came back in time, so nobody is told it is gone.
    vi.advanceTimersByTime(CONTEXT_RESTORE_WAIT_MS * 2)
    expect(gone).not.toHaveBeenCalled()
  })

  it("reports the context gone when it does not come back in time", () => {
    const { renderer, lose } = setUp()
    const gone = vi.fn()
    renderer.onLost = gone
    lose()
    vi.advanceTimersByTime(CONTEXT_RESTORE_WAIT_MS - 1)
    expect(gone).not.toHaveBeenCalled()
    vi.advanceTimersByTime(1)
    expect(gone).toHaveBeenCalledTimes(1)
  })

  it("reports the context gone when it comes back unusable", () => {
    const { gl, renderer, batch, lose, restore } = setUp()
    const restored = vi.fn()
    const gone = vi.fn()
    renderer.onRestored = restored
    renderer.onLost = gone
    lose()
    gl.breakPrograms()
    restore()
    expect(gone).toHaveBeenCalledTimes(1)
    expect(restored).not.toHaveBeenCalled()
    drawFrame(renderer, batch)
    expect(gl.count("drawArraysInstanced")).toBe(0)
    // Told once, not again when the wait runs out.
    vi.advanceTimersByTime(CONTEXT_RESTORE_WAIT_MS * 2)
    expect(gone).toHaveBeenCalledTimes(1)
  })

  it("gives its context back when it is disposed, and stops waiting", () => {
    const { canvas, gl, renderer, lose } = setUp()
    const gone = vi.fn()
    renderer.onLost = gone
    lose()
    gl.restore()
    renderer.dispose()
    expect(gl.context.isContextLost()).toBe(true)
    vi.advanceTimersByTime(CONTEXT_RESTORE_WAIT_MS * 2)
    expect(gone).not.toHaveBeenCalled()
    // Its listeners are gone too.
    const event = new Event("webglcontextlost", { cancelable: true })
    canvas.dispatchEvent(event)
    expect(event.defaultPrevented).toBe(false)
  })

  it("gives the context back when it cannot build its program", () => {
    const gl = createFakeGl()
    gl.breakPrograms()
    expect(() => setUp(gl)).toThrow(RendererUnavailableError)
    expect(gl.context.isContextLost()).toBe(true)
  })
})
