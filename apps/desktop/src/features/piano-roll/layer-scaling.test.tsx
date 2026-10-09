import { act, cleanup, fireEvent, render, screen } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { rgbaToCss, rgbFromInt, TimeGridView } from "@/lib/canvas"
import { createFake2D, type Fake2D } from "@/lib/canvas/test-utils"
import { dispatch } from "@/lib/store/project"
import { applyUiScale, uiScaleFactor } from "@/lib/ui-scale"
import { settle } from "@/test/harness"

import { SessionContext } from "./context"
import { Ruler } from "./ruler"
import { channel, currentPattern, notesOf, startRoll } from "./test-utils"
import { ValueLane } from "./value-lane"

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { error: vi.fn(), success: vi.fn() }),
}))

const observers = new Map<Element, ResizeObserverCallback>()
const contexts = new Map<HTMLCanvasElement, Fake2D>()
const frames = new Map<number, FrameRequestCallback>()
let frameId = 0
let width = 4000
let height = 2000
let stopFixture: (() => void) | null = null
const origin = { x: 110, y: 70 }

class Observer {
  private callback: ResizeObserverCallback
  constructor(callback: ResizeObserverCallback) {
    this.callback = callback
  }
  observe(element: Element) {
    observers.set(element, this.callback)
  }
  disconnect() {}
  unobserve() {}
}

beforeEach(() => {
  vi.stubGlobal("ResizeObserver", Observer)
  vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => {
    frames.set(++frameId, callback)
    return frameId
  })
  vi.stubGlobal("cancelAnimationFrame", (id: number) => frames.delete(id))
  // Layout is supplied explicitly: jsdom has no layout or canvas rasterizer.
  vi.spyOn(
    HTMLCanvasElement.prototype,
    "clientWidth",
    "get"
  ).mockImplementation(() => width)
  vi.spyOn(
    HTMLCanvasElement.prototype,
    "clientHeight",
    "get"
  ).mockImplementation(function (this: HTMLCanvasElement) {
    return this.getAttribute("aria-label")?.startsWith("Time ruler") ? 21 : 84
  })
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockImplementation(
    function (this: HTMLCanvasElement) {
      let fake = contexts.get(this)
      if (!fake) {
        fake = createFake2D()
        Object.assign(fake.context, {
          setTransform: () => {
            fake!.fills.length = 0
          },
          clearRect: vi.fn(() => {
            fake!.fills.length = 0
          }),
          fillText() {},
          beginPath() {},
          moveTo() {},
          lineTo() {},
          closePath() {},
          fill() {},
        })
        contexts.set(this, fake)
      }
      return fake.context
    }
  )
})

afterEach(() => {
  cleanup()
  stopFixture?.()
  stopFixture = null
  applyUiScale(100)
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
  observers.clear()
  contexts.clear()
  frames.clear()
  document.body.replaceChildren()
})

function paintFrames() {
  act(() => {
    const pending = [...frames.values()]
    frames.clear()
    for (const callback of pending) callback(0)
  })
}

function resize(element: Element) {
  observers.get(element)!([], {} as ResizeObserver)
}

function pointer(
  canvas: HTMLCanvasElement,
  type: string,
  x: number,
  y: number
) {
  const bounds = canvas.getBoundingClientRect()
  const event = new MouseEvent(type, {
    bubbles: true,
    button: 0,
    clientX: bounds.left + x * uiScaleFactor(),
    clientY: bounds.top + y * uiScaleFactor(),
  })
  Object.defineProperty(event, "pointerId", { value: 1 })
  fireEvent(canvas, event)
}

async function start(
  widthCss: number,
  heightCss: number,
  scale: number,
  dpr: number
) {
  const roll = await startRoll()
  stopFixture = () => roll.stop()
  await dispatch({ type: "addChannel", name: "Lead" })
  await dispatch({
    type: "addNotes",
    pattern: currentPattern().id,
    channel: channel("Lead").id,
    notes: [{ start: 3840, length: 960, key: 60, velocity: 0.5 }],
  })
  roll.show("Lead")
  width = widthCss
  height = heightCss
  vi.stubGlobal("devicePixelRatio", dpr)
  applyUiScale(scale)
  const container = document.createElement("div")
  Object.defineProperties(container, {
    clientWidth: { get: () => width },
    clientHeight: { get: () => height },
    clientLeft: { value: 2 },
    clientTop: { value: 3 },
  })
  container.getBoundingClientRect = () =>
    new DOMRect(
      origin.x,
      origin.y,
      (width + 4) * uiScaleFactor(),
      (height + 6) * uiScaleFactor()
    )
  document.body.append(container)
  const view = await TimeGridView.create(container, {
    renderer: "canvas2d",
    autoRender: false,
    initial: { pxPerTick: 0.08, rowHeight: 16, scrollRow: 0 },
  })
  stopFixture = () => {
    roll.session.attachView(null)
    view.destroy()
    roll.stop()
  }
  const color = channel("Lead").color
  roll.editor.setPalette([rgbFromInt(color)])
  roll.session.attachView(view)
  const mounted = render(
    <SessionContext.Provider value={roll.session}>
      <Ruler />
      <ValueLane kind="velocity" color={color} />
    </SessionContext.Provider>
  )
  const ruler = screen.getByLabelText(/^Time ruler/) as HTMLCanvasElement
  const lane = screen.getByLabelText("Note velocity values") as HTMLCanvasElement
  for (const canvas of [ruler, lane]) {
    canvas.getBoundingClientRect = () =>
      new DOMRect(
        origin.x + 2 * uiScaleFactor(),
        origin.y + 3 * uiScaleFactor(),
        width * uiScaleFactor(),
        canvas.clientHeight * uiScaleFactor()
      )
  }
  paintFrames()
  return {
    roll,
    view,
    container,
    ruler,
    lane,
    color,
    stop() {
      mounted.unmount()
      stopFixture?.()
      stopFixture = null
    },
  }
}

type Fixture = Awaited<ReturnType<typeof start>>

function renderedCap(f: Fixture) {
  const fill = contexts
    .get(f.lane)!
    .fills.find(
      ({ style, rect }) =>
        style === rgbaToCss(rgbFromInt(f.color)) && rect[2] > rect[3]
    )
  expect(fill, "the actual value-lane painter draws the note cap").toBeDefined()
  return fill!.rect[0] / (f.lane.width / width)
}

function renderedMarker(f: Fixture) {
  const density = f.ruler.width / width
  // Locate the real full-height end marker and the note's painted cap.
  const marker = contexts
    .get(f.ruler)!
    .fills.find(
      ({ style, rect }) =>
        style === rgbaToCss(f.view.theme.item) &&
        rect[1] === 0 &&
        rect[3] === f.ruler.height
    )
  expect(marker).toBeDefined()
  return (marker!.rect[0] + marker!.rect[2] / 2) / density
}

function checkAlignment(f: Fixture, expectedX = 307.2) {
  f.view.flush()
  paintFrames()
  const markerX = renderedMarker(f)
  const capX = renderedCap(f)
  // Tick 3840 at the configured 0.08 px/tick is logical X 307.2.
  // Expectations do not use the implementation's device-transform helper.
  expect(markerX).toBeCloseTo(expectedX, 0)
  expect(capX).toBeCloseTo(expectedX, 0)
  pointer(f.ruler, "pointermove", markerX, 10)
  expect(f.ruler.style.cursor).toBe("ew-resize")
  const point = f.view.localPoint({
    clientX: origin.x + (2 + capX + 6) * uiScaleFactor(),
    clientY: origin.y + (3 + 67 * 16 + 8) * uiScaleFactor(),
  })
  expect(f.view.hitTest(point.x, point.y)?.id).toBe(notesOf("Lead")[0].id)
  const gridFills = contexts.get(f.view.element)!.fills
  const noteFill = gridFills.find(
    ({ style }) => style === rgbaToCss(rgbFromInt(f.color))
  )
  expect(noteFill).toBeDefined()
  // The filled note starts inside its one-device-pixel border.
  const gridX =
    (noteFill!.rect[0] - f.view.transform.lineWidth) / f.view.viewport.dpr
  expect(Math.abs(gridX - capX)).toBeLessThan(1)
  for (const canvas of [f.view.element, f.ruler, f.lane]) {
    expect(canvas.width).toBeLessThanOrEqual(8192)
    expect(canvas.height).toBeLessThanOrEqual(8192)
    expect(canvas.width * canvas.height).toBeLessThanOrEqual(16_777_216)
  }
}

describe("real piano layer painters and input under independent density caps", () => {
  it.each([
    [4000, 2000, 200, 2],
    [5000, 5000, 200, 2],
    [4000, 2000, 200, 1.25],
    [5000, 5000, 200, 1.5],
    [4000, 2000, 125, 2],
    [800, 1200, 100, 1],
  ])(
    "%ix%i at scale %i / DPR %s aligns rendered markers and edits the visible bar",
    async (w, h, scale, dpr) => {
      const f = await start(w, h, scale, dpr)
      try {
        checkAlignment(f)
        const x = renderedCap(f)
        pointer(f.lane, "pointerdown", x + 1, 5)
        pointer(f.lane, "pointerup", x + 1, 5)
        await act(settle)
        expect(notesOf("Lead")[0].velocity).toBe(1)
        pointer(f.lane, "pointerdown", renderedCap(f) + 1, 79)
        pointer(f.lane, "pointerup", renderedCap(f) + 1, 79)
        await act(settle)
        expect(notesOf("Lead")[0].velocity).toBe(0)
      } finally {
        f.stop()
      }
    }
  )

  it("drags the rendered end marker through the real ruler command path", async () => {
    const f = await start(4000, 2000, 200, 2)
    try {
      checkAlignment(f)
      pointer(f.ruler, "pointerdown", renderedMarker(f), 10)
      pointer(f.ruler, "pointermove", 614.4, 10)
      pointer(f.ruler, "pointerup", 614.4, 10)
      await act(settle)
      paintFrames()
      expect(currentPattern().lengthSteps).toBe(32)
      expect(renderedMarker(f)).toBeCloseTo(614.4, 0)
      expect(notesOf("Lead")[0].start).toBe(3840)
    } finally {
      f.stop()
    }
  })

  it("repaints on observer-only logical resize even when capped backing sizes stay equal", async () => {
    const f = await start(5000, 5000, 200, 2)
    try {
      const before = [f.ruler.width, f.ruler.height]
      const clear = vi.mocked(contexts.get(f.ruler)!.context.clearRect)
      clear.mockClear()
      width = 5001
      // The layer observer fires first: it must not rely on a grid notification.
      act(() => resize(f.ruler))
      expect([f.ruler.width, f.ruler.height]).toEqual(before)
      expect(clear).toHaveBeenCalledOnce()
      act(() => {
        resize(f.lane)
        resize(f.container)
      })
      checkAlignment(f)
      act(() => {
        applyUiScale(75)
        window.dispatchEvent(new Event("resize"))
      })
      checkAlignment(f)
      act(() => {
        vi.stubGlobal("devicePixelRatio", 1.25)
        window.dispatchEvent(new Event("resize"))
      })
      checkAlignment(f)
      width = 800
      height = 1200
      act(() => {
        resize(f.container)
        resize(f.ruler)
        resize(f.lane)
      })
      act(() => applyUiScale(100))
      checkAlignment(f)
      expect(f.ruler.width).toBe(1000)
      act(() => f.view.setViewport({ ...f.view.viewport, scrollTick: 27.25 }))
      checkAlignment(f, 305.02)
    } finally {
      f.stop()
    }
  })
})
