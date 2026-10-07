import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { rgba } from "./color"
import * as core from "./index"
import * as reactEntry from "./react"
import { RECT_SELECTED, RectBatch } from "./rect-batch"
import { CONTEXT_RESTORE_WAIT_MS } from "./renderer-webgl2"
import { indexBatch } from "./spatial-index"
import {
  createFake2D,
  createFakeGl,
  type Fake2D,
  type FakeGl,
} from "./test-utils"
import { TimeGridView, type TimeGridViewOptions } from "./time-grid-view"

// One fake context per canvas and kind, as a browser hands them out.
const drawn2D = new WeakMap<HTMLCanvasElement, Fake2D>()
let gl: FakeGl

beforeEach(() => {
  gl = createFakeGl()
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockImplementation(
    function (this: HTMLCanvasElement, kind: string) {
      if (kind === "webgl2") return gl.context
      let fake = drawn2D.get(this)
      if (!fake) {
        fake = createFake2D()
        drawn2D.set(this, fake)
      }
      return fake.context
    }
  )
})

afterEach(() => {
  vi.restoreAllMocks()
  vi.useRealTimers()
  document.body.replaceChildren()
})

async function createView(options: TimeGridViewOptions = {}) {
  const container = document.createElement("div")
  Object.defineProperty(container, "clientWidth", { value: 800 })
  Object.defineProperty(container, "clientHeight", { value: 400 })
  document.body.append(container)
  const view = await TimeGridView.create(container, {
    autoRender: false,
    ...options,
  })
  const batch = new RectBatch()
  batch.push(1, 960, 960, 3, 1, rgba(200, 100, 50), RECT_SELECTED)
  batch.push(2, 1920, 960, 3, 1, rgba(200, 100, 50))
  view.setItems(indexBatch(batch))
  view.flush()
  return { view, container, batch }
}

describe("TimeGridView layers", () => {
  it("redraws the overlay with the items while a selection is dragged", async () => {
    const { view } = await createView({ renderer: "canvas2d" })
    const painter = vi.fn()
    view.addOverlayPainter(painter)
    view.flush()
    painter.mockClear()

    view.setDragOffset(240, 1)
    expect(view.flush()).toMatchObject({ base: true, overlay: true })
    expect(painter).toHaveBeenCalledTimes(1)

    view.setDragResize(0, 480, 240)
    expect(view.flush()).toMatchObject({ base: true, overlay: true })
    expect(painter).toHaveBeenCalledTimes(2)

    // The same offset again changes nothing and draws nothing.
    view.setDragOffset(240, 1)
    view.setDragResize(0, 480, 240)
    expect(view.flush()).toBeNull()

    view.setDragOffset(0, 0)
    view.setDragResize(0, 0)
    expect(view.flush()).toMatchObject({ base: true, overlay: true })
    expect(painter).toHaveBeenCalledTimes(3)
  })

  it("redraws the overlay when the items are replaced", async () => {
    const { view, batch } = await createView({ renderer: "canvas2d" })
    const painter = vi.fn()
    view.addOverlayPainter(painter)
    view.flush()
    painter.mockClear()
    view.setItems(indexBatch(batch))
    expect(view.flush()).toMatchObject({ base: true, overlay: true })
    expect(painter).toHaveBeenCalledTimes(1)
  })

  it("leaves the items alone for the playhead and the marquee", async () => {
    const { view } = await createView({ renderer: "canvas2d" })
    view.setPlayhead(960)
    expect(view.flush()).toMatchObject({ base: false, overlay: true })
    view.setMarquee({ tick0: 0, row0: 0, tick1: 960, row1: 4 })
    expect(view.flush()).toMatchObject({ base: false, overlay: true })
  })

  it("leaves the overlay alone for the grid and the underlay", async () => {
    const { view } = await createView({ renderer: "canvas2d" })
    view.setTimeGrid({ ticksPerStep: 120, stepsPerBeat: 4, beatsPerBar: 3 })
    expect(view.flush()).toMatchObject({ base: true, overlay: false })
    view.setUnderlay(indexBatch(new RectBatch()))
    expect(view.flush()).toMatchObject({ base: true, overlay: false })
  })
})

describe("TimeGridView after a lost WebGL context", () => {
  function lose(view: TimeGridView) {
    gl.lose()
    view.element.dispatchEvent(
      new Event("webglcontextlost", { cancelable: true })
    )
  }

  it("redraws with WebGL when the context comes back", async () => {
    const { view } = await createView({ renderer: "webgl2" })
    expect(view.renderer.info.kind).toBe("webgl2")
    const draws = gl.count("drawArraysInstanced")
    expect(draws).toBeGreaterThan(0)

    vi.useFakeTimers()
    lose(view)
    gl.restore()
    view.element.dispatchEvent(new Event("webglcontextrestored"))
    expect(view.flush()).toMatchObject({ base: true, overlay: true })
    expect(gl.count("drawArraysInstanced")).toBeGreaterThan(draws)
    vi.advanceTimersByTime(CONTEXT_RESTORE_WAIT_MS * 2)
    expect(view.renderer.info.kind).toBe("webgl2")
  })

  it("carries on in Canvas 2D when the context does not come back", async () => {
    const { view, container } = await createView({ renderer: "webgl2" })
    const element = view.element
    const lostRenderer = view.renderer

    vi.useFakeTimers()
    lose(view)
    view.flush()
    vi.advanceTimersByTime(CONTEXT_RESTORE_WAIT_MS)

    expect(view.renderer).not.toBe(lostRenderer)
    expect(view.renderer.info.kind).toBe("canvas2d")
    // The canvas that takes the pointer is still the same one, on top of
    // the one that is drawn into now.
    expect(view.element).toBe(element)
    expect(element.isConnected).toBe(true)
    expect(element.style.opacity).toBe("0")
    const canvases = [...container.querySelectorAll("canvas")]
    expect(canvases).toHaveLength(3)
    expect(canvases[1]).toBe(element)
    expect(canvases[0].style.pointerEvents).toBe("none")

    // The same items are drawn again, by the new renderer.
    expect(view.flush()).toMatchObject({ base: true, overlay: true })
    const fills = drawn2D.get(canvases[0])?.fills ?? []
    expect(fills.some((fill) => fill.rect.join() === "60,49,60,15")).toBe(true)

    // A context that shows up late changes nothing.
    gl.restore()
    element.dispatchEvent(new Event("webglcontextrestored"))
    expect(view.renderer.info.kind).toBe("canvas2d")
    expect(view.flush()).toBeNull()
  })

  it("removes every canvas when it is destroyed after falling back", async () => {
    const { view, container } = await createView({ renderer: "webgl2" })
    vi.useFakeTimers()
    lose(view)
    vi.advanceTimersByTime(CONTEXT_RESTORE_WAIT_MS)
    view.destroy()
    expect(container.querySelectorAll("canvas")).toHaveLength(0)
  })
})

describe("the two entries", () => {
  it("keeps the React wrapper out of the core entry", () => {
    expect(typeof reactEntry.TimeGridCanvas).toBe("function")
    expect("TimeGridCanvas" in core).toBe(false)
    expect(core.TimeGridView).toBe(TimeGridView)
  })
})
