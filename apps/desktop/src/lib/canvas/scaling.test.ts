import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { applyUiScale } from "@/lib/ui-scale"
import { observeCanvas } from "@/components/audio/canvas"
import { rgba } from "./color"
import { createPointerFrame } from "./pointer-frame"
import { RectBatch } from "./rect-batch"
import { indexBatch } from "./spatial-index"
import { createFake2D } from "./test-utils"
import { TimeGridView } from "./time-grid-view"

const resizes = new Map<Element, ResizeObserverCallback>()
class Observer {
  private callback: ResizeObserverCallback
  constructor(callback: ResizeObserverCallback) {
    this.callback = callback
  }
  observe(element: Element) {
    resizes.set(element, this.callback)
  }
  disconnect() {}
  unobserve() {}
}
beforeEach(() => {
  vi.stubGlobal("ResizeObserver", Observer)
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockImplementation(
    () => createFake2D().context
  )
})
afterEach(() => {
  applyUiScale(100)
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
  resizes.clear()
  document.body.replaceChildren()
})

const cases = [
  [75, 1, 600, 240],
  [75, 2, 1200, 480],
  [75, 1.25, 750, 300],
  [125, 1, 1000, 400],
  [125, 2, 2000, 800],
  [125, 1.25, 1250, 500],
  [150, 1, 1200, 480],
  [150, 2, 2400, 960],
  [150, 1.25, 1500, 600],
  [200, 1, 1600, 640],
  [200, 2, 3200, 1280],
  [200, 1.25, 2000, 800],
]

async function scaledView(scale: number, dpr = 1) {
  applyUiScale(scale)
  vi.stubGlobal("devicePixelRatio", dpr)
  const container = document.createElement("div")
  Object.defineProperties(container, {
    clientWidth: { value: 800, configurable: true },
    clientHeight: { value: 320, configurable: true },
  })
  let top = 50
  container.getBoundingClientRect = () =>
    new DOMRect(100, top, (800 * scale) / 100, (320 * scale) / 100)
  document.body.append(container)
  const view = await TimeGridView.create(container, {
    renderer: "canvas2d",
    autoRender: false,
    initial: { scrollRow: 57, pxPerTick: 0.05, rowHeight: 16 },
  })
  return {
    view,
    container,
    moveLayout: () => {
      top = 150
    },
  }
}

describe("real common canvas at application scale and monitor density", () => {
  it.each(cases)(
    "%i%% at DPR %s draws %ix%i and hits the same note",
    async (scale, dpr, width, height) => {
      const { view, container, moveLayout } = await scaledView(scale, dpr)
      try {
        const batch = new RectBatch()
        batch.push(7, 960, 240, 67, 1, rgba(200, 100, 50))
        view.setItems(indexBatch(batch))
        view.flush()
        expect([view.element.width, view.element.height]).toEqual([
          width,
          height,
        ])
        const event = {
          clientX: 100 + (54 * scale) / 100,
          clientY: 50 + (168 * scale) / 100,
        }
        const point = view.localPoint(event)
        expect(view.hitTest(point.x, point.y)?.id).toBe(7)
        const frame = createPointerFrame((e) => view.localPoint(e))
        frame.hold(event)
        moveLayout()
        expect(frame.point(event)).toEqual(point)
        const moved = frame.point({
          clientX: event.clientX + (24 * scale) / 100,
          clientY: event.clientY - (16 * scale) / 100,
        })
        expect(moved.x).toBeCloseTo(point.x + 24)
        expect(moved.y).toBeCloseTo(point.y - 16)
        frame.release()
        // A rounded or unzoomed observer device box must not overrule effective density.
        const callback = resizes.get(container)!
        callback(
          [
            {
              devicePixelContentBoxSize: [{ inlineSize: 800, blockSize: 320 }],
            } as unknown as ResizeObserverEntry,
          ],
          {} as ResizeObserver
        )
        expect([view.element.width, view.element.height]).toEqual([
          width,
          height,
        ])
        Object.defineProperty(container, "clientWidth", { value: 400 })
        callback(
          [{ devicePixelContentBoxSize: [] } as unknown as ResizeObserverEntry],
          {} as ResizeObserver
        )
        expect(view.viewport.width).toBe(400)
        expect(view.element.width).toBe(width / 2)
      } finally {
        view.destroy()
      }
    }
  )

  it("updates density without a ResizeObserver delivery, bounds huge stores, and matches the audio canvas contract", async () => {
    const { view, container } = await scaledView(100, 2)
    try {
      applyUiScale(200)
      expect(view.element.width).toBe(3200)
      expect(view.viewport.dpr).toBe(4)
      Object.defineProperties(container, {
        clientWidth: { value: 20000 },
        clientHeight: { value: 20000 },
      })
      window.dispatchEvent(new Event("resize"))
      expect(view.element.width * view.element.height).toBeLessThanOrEqual(
        16_777_216
      )
      expect(view.element.width).toBeLessThanOrEqual(8192)
      Object.defineProperty(container, "clientHeight", { value: 18000 })
      window.dispatchEvent(new Event("resize"))
      expect(view.element.width * view.element.height).toBeLessThanOrEqual(
        16_777_216
      )
      const canvas = document.createElement("canvas")
      canvas.getBoundingClientRect = () => new DOMRect(0, 0, 200, 100)
      const changed = vi.fn()
      const stop = observeCanvas(canvas, changed)
      expect(changed).toHaveBeenLastCalledWith({
        width: 100,
        height: 50,
        pixelWidth: 400,
        pixelHeight: 200,
        dpr: 4,
      })
      stop()
    } finally {
      view.destroy()
    }
  })
})
