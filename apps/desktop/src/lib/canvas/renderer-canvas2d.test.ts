import { describe, expect, it } from "vitest"

import { rgba, rgbaToCss } from "./color"
import {
  RECT_FLAT,
  RECT_FULL_HEIGHT,
  RECT_SELECTED,
  RECT_VLINE,
  RectBatch,
} from "./rect-batch"
import { createCanvas2DRenderer } from "./renderer-canvas2d"
import { visibleRange, indexBatch } from "./spatial-index"
import { deriveGridTheme } from "./theme"
import { deviceTransform, visibleTicks, type Viewport } from "./viewport"

const theme = deriveGridTheme({
  background: rgba(20, 20, 20),
  foreground: rgba(250, 250, 250),
  mutedForeground: rgba(160, 160, 160),
  brand: rgba(230, 60, 140),
  playhead: rgba(90, 160, 255),
  gridLine: rgba(255, 255, 255, 18),
  gridLineStrong: rgba(255, 255, 255, 51),
})

const NOTE = rgba(200, 100, 50)

interface Fill {
  style: string
  rect: [number, number, number, number]
}

// jsdom has no canvas implementation, and the calls are what matter here:
// they are the pixels a real context would fill.
function recordingCanvas(width: number, height: number) {
  const fills: Fill[] = []
  const context = {
    fillStyle: "",
    setTransform() {},
    fillRect(x: number, y: number, w: number, h: number) {
      fills.push({ style: this.fillStyle, rect: [x, y, w, h] })
    },
  }
  const canvas = document.createElement("canvas")
  canvas.width = width
  canvas.height = height
  Object.defineProperty(canvas, "getContext", { value: () => context })
  return { canvas, fills }
}

function viewportAt(dpr: number): Viewport {
  return {
    width: 800,
    height: 400,
    dpr,
    scrollTick: 0,
    scrollRow: 0,
    pxPerTick: 0.0625,
    rowHeight: 16,
  }
}

function draw(
  batch: RectBatch,
  viewport: Viewport,
  options: {
    first?: number
    last?: number
    dragTicks?: number
    dragRows?: number
  } = {}
): Fill[] {
  const transform = deviceTransform(viewport)
  const { canvas, fills } = recordingCanvas(
    transform.widthDev,
    transform.heightDev
  )
  const renderer = createCanvas2DRenderer(canvas)
  renderer.setTheme(theme)
  renderer.beginFrame(transform)
  renderer.drawBatch(batch, options)
  renderer.endFrame()
  // The first fill is the background clear.
  return fills.slice(1)
}

describe("Canvas 2D renderer", () => {
  it("clears to the background color", () => {
    const transform = deviceTransform(viewportAt(1))
    const { canvas, fills } = recordingCanvas(800, 400)
    const renderer = createCanvas2DRenderer(canvas)
    renderer.setTheme(theme)
    renderer.beginFrame(transform)
    renderer.endFrame()
    expect(fills).toEqual([{ style: "rgb(20,20,20)", rect: [0, 0, 800, 400] }])
  })

  it("aligns a note to its row, below the row line, with a border", () => {
    const batch = new RectBatch()
    // One beat at beat 2 on row 3: x 60 to 120, row top at y 48.
    batch.push(1, 960, 960, 3, 1, NOTE)
    expect(draw(batch, viewportAt(1))).toEqual([
      { style: "rgb(124,62,31)", rect: [60, 49, 60, 15] },
      { style: "rgb(200,100,50)", rect: [61, 50, 58, 13] },
    ])
  })

  it("doubles every measure at a device pixel ratio of 2", () => {
    const batch = new RectBatch()
    batch.push(1, 960, 960, 3, 1, NOTE)
    expect(draw(batch, viewportAt(2)).map((f) => f.rect)).toEqual([
      [120, 98, 120, 30],
      [122, 100, 116, 26],
    ])
  })

  it("draws grid lines one CSS pixel wide over the full height", () => {
    const batch = new RectBatch()
    const flags = RECT_FLAT | RECT_FULL_HEIGHT | RECT_VLINE
    batch.push(-1, 3840, 0, 0, 0, theme.gridBar, flags)
    expect(draw(batch, viewportAt(1))).toEqual([
      { style: rgbaToCss(theme.gridBar), rect: [240, 0, 1, 400] },
    ])
    expect(draw(batch, viewportAt(2))[0].rect).toEqual([480, 0, 2, 800])
    expect(draw(batch, viewportAt(1.5))[0].rect).toEqual([360, 0, 2, 600])
  })

  it("keeps a note narrower than a pixel visible and unbordered", () => {
    const batch = new RectBatch()
    batch.push(1, 4000, 240, 2, 1, NOTE)
    const fills = draw(batch, { ...viewportAt(1), pxPerTick: 0.0025 })
    expect(fills).toEqual([{ style: "rgb(200,100,50)", rect: [10, 33, 1, 15] }])
  })

  it("skips rects outside the canvas", () => {
    const batch = new RectBatch()
    batch.push(1, 960, 240, 30, 1, NOTE)
    batch.push(2, 20_000, 240, 3, 1, NOTE)
    batch.push(3, 960, 240, 3, 1, NOTE)
    const fills = draw(batch, viewportAt(1))
    // Row 30 is below the 400 px canvas and tick 20,000 is right of it.
    expect(fills).toHaveLength(2)
    expect(fills[0].rect[0]).toBe(60)
  })

  it("draws only the index range it is given", () => {
    const batch = new RectBatch()
    for (let i = 0; i < 100; i++) batch.push(i, i * 240, 240, 1, 1, NOTE)
    const items = indexBatch(batch)
    const viewport = { ...viewportAt(1), scrollTick: 4800 }
    const ticks = visibleTicks(viewport)
    const range = visibleRange(items, ticks.start, ticks.end)
    const fills = draw(items.batch, viewport, range)
    // 800 px at 15 px per step shows 54 steps, each a border and a fill.
    expect(range.last - range.first).toBeLessThan(60)
    expect(fills.length).toBe(54 * 2)
  })

  it("groups fills by color so each style is set once", () => {
    const batch = new RectBatch()
    const other = rgba(10, 200, 10)
    for (let i = 0; i < 20; i++) {
      batch.push(i, i * 480, 240, 1, 1, i % 2 ? NOTE : other)
    }
    const fills = draw(batch, viewportAt(1))
    let changes = 0
    for (let i = 1; i < fills.length; i++) {
      if (fills[i].style !== fills[i - 1].style) changes++
    }
    // Two colors, each with a border style and a fill style.
    expect(changes).toBe(3)
  })

  it("draws selected rects last, moved by the drag offset", () => {
    const batch = new RectBatch()
    batch.push(1, 960, 960, 3, 1, NOTE, RECT_SELECTED)
    batch.push(2, 1920, 960, 3, 1, NOTE)
    const fills = draw(batch, viewportAt(1), { dragTicks: 240, dragRows: 2 })
    expect(fills).toEqual([
      { style: "rgb(124,62,31)", rect: [120, 49, 60, 15] },
      { style: "rgb(200,100,50)", rect: [121, 50, 58, 13] },
      { style: rgbaToCss(theme.selectionBorder), rect: [75, 81, 60, 15] },
      { style: "rgb(220,160,130)", rect: [76, 82, 58, 13] },
    ])
  })
})
