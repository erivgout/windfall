import { describe, expect, it } from "vitest"

import { rgba, type Rgba } from "./color"
import { DEFAULT_TIME_GRID, pianoRows, writeGrid } from "./grid"
import {
  RECT_FLAT,
  RECT_FULL_HEIGHT,
  RECT_FULL_WIDTH,
  RECT_HLINE,
  RECT_SELECTED,
  RECT_VLINE,
  RectBatch,
} from "./rect-batch"
import {
  BORDER_ROUNDING,
  BORDER_SHADE,
  resizedSpan,
  type DrawOptions,
} from "./renderer"
import { createCanvas2DRenderer } from "./renderer-canvas2d"
import { indexBatch } from "./spatial-index"
import { deriveGridTheme } from "./theme"
import {
  deviceTransform,
  type DeviceTransform,
  type Viewport,
} from "./viewport"

/*
 * The Canvas 2D renderer against the GPU renderers, without a GPU. One side
 * is the vertex and fragment shaders written out in plain arithmetic and
 * rasterized here. The other is the fills the Canvas 2D renderer issues,
 * painted with the same source-over blend a 2D context uses. Both must end
 * up with the same picture.
 */

const theme = deriveGridTheme({
  background: rgba(20, 20, 24),
  foreground: rgba(250, 250, 250),
  mutedForeground: rgba(160, 160, 160),
  brand: rgba(230, 60, 140),
  playhead: rgba(90, 160, 255),
  gridLine: rgba(255, 255, 255, 18),
  gridLineStrong: rgba(255, 255, 255, 51),
})

const viewport: Viewport = {
  width: 240,
  height: 96,
  dpr: 1,
  scrollTick: 0,
  scrollRow: 0,
  pxPerTick: 0.0625,
  rowHeight: 16,
}

type Scene = { batch: RectBatch; options?: DrawOptions }[]

class Picture {
  readonly width: number
  readonly height: number
  /** Red, green and blue per pixel, 0 to 255, not rounded. */
  readonly data: Float64Array

  constructor(width: number, height: number, background: Rgba) {
    this.width = width
    this.height = height
    this.data = new Float64Array(width * height * 3)
    for (let i = 0; i < width * height; i++) {
      this.data[i * 3] = background.r
      this.data[i * 3 + 1] = background.g
      this.data[i * 3 + 2] = background.b
    }
  }

  /** Source over: what both a 2D context and the GPU blend state do. */
  blend(x: number, y: number, color: Rgba): void {
    if (x < 0 || y < 0 || x >= this.width || y >= this.height) return
    const i = (y * this.width + x) * 3
    const alpha = color.a / 255
    this.data[i] = color.r * alpha + this.data[i] * (1 - alpha)
    this.data[i + 1] = color.g * alpha + this.data[i + 1] * (1 - alpha)
    this.data[i + 2] = color.b * alpha + this.data[i + 2] * (1 - alpha)
  }
}

const snap = (value: number) => Math.floor(value + 0.5)

/** One draw call of the GPU renderers: the shaders in renderer-webgl2.ts. */
function shadePass(
  picture: Picture,
  batch: RectBatch,
  t: DeviceTransform,
  pass: "all" | "unselected" | "selected",
  options: DrawOptions
): void {
  const lw = t.lineWidth
  for (let i = 0; i < batch.count; i++) {
    const flags = batch.flags[i]
    const selected = (flags & RECT_SELECTED) !== 0
    if (pass === "unselected" && selected) continue
    if (pass === "selected" && !selected) continue
    const flat = (flags & RECT_FLAT) !== 0
    let start = batch.start(i) - t.scrollTick
    let length = batch.length(i)
    let row = batch.row(i)
    if (selected && pass !== "all") {
      const span = resizedSpan(
        start,
        length,
        options.resizeStart ?? 0,
        options.resizeEnd ?? 0,
        options.minLength ?? 0
      )
      start = span.start + (options.dragTicks ?? 0)
      length = span.length
      row += options.dragRows ?? 0
    }
    let x0 = snap(start * t.scaleX - t.offsetX)
    let x1 = snap((start + length) * t.scaleX - t.offsetX)
    let y0 = snap(row * t.scaleY - t.offsetY)
    let y1 = snap((row + batch.rowSpan(i)) * t.scaleY - t.offsetY)
    if (!flat) y0 += lw
    if (flags & RECT_FULL_WIDTH) {
      x0 = 0
      x1 = picture.width
    }
    if (flags & RECT_FULL_HEIGHT) {
      y0 = 0
      y1 = picture.height
    }
    if (flags & RECT_VLINE) x1 = x0 + lw
    if (flags & RECT_HLINE) y1 = y0 + lw
    x1 = Math.max(x1, x0 + lw)
    y1 = Math.max(y1, y0 + lw)
    const width = x1 - x0
    const height = y1 - y0

    const c = i * 4
    const alpha = batch.colors[c + 3]
    let fill = [batch.colors[c], batch.colors[c + 1], batch.colors[c + 2]]
    let border = fill.map((channel) =>
      Math.floor(channel * BORDER_SHADE + BORDER_ROUNDING)
    )
    if (selected) {
      const to = theme.selectionFill
      fill = [
        fill[0] + (to.r - fill[0]) * theme.selectionMix,
        fill[1] + (to.g - fill[1]) * theme.selectionMix,
        fill[2] + (to.b - fill[2]) * theme.selectionMix,
      ]
      const frame = theme.selectionBorder
      border = [frame.r, frame.g, frame.b]
    }
    const borderWidth = flat || width < 3 * lw || height < 3 * lw ? 0 : lw

    for (let y = y0; y < y1; y++) {
      for (let x = x0; x < x1; x++) {
        const localX = x + 0.5 - x0
        const localY = y + 0.5 - y0
        const edge = Math.min(
          Math.min(localX, width - localX),
          Math.min(localY, height - localY)
        )
        const [r, g, b] = edge < borderWidth ? border : fill
        picture.blend(x, y, { r, g, b, a: alpha })
      }
    }
  }
}

function drawOnGpu(scene: Scene, t: DeviceTransform): Picture {
  const picture = new Picture(t.widthDev, t.heightDev, theme.background)
  for (const { batch, options = {} } of scene) {
    if (batch.selectedCount === 0) {
      shadePass(picture, batch, t, "all", options)
    } else {
      shadePass(picture, batch, t, "unselected", options)
      shadePass(picture, batch, t, "selected", options)
    }
  }
  return picture
}

function parseStyle(style: string): Rgba {
  const parts = style
    .slice(style.indexOf("(") + 1, -1)
    .split(",")
    .map(Number)
  const [r, g, b, alpha = 1] = parts
  return { r, g, b, a: alpha * 255 }
}

function drawOnCanvas2D(scene: Scene, t: DeviceTransform): Picture {
  const picture = new Picture(t.widthDev, t.heightDev, theme.background)
  const context = {
    fillStyle: "",
    setTransform() {},
    fillRect(x: number, y: number, width: number, height: number) {
      const color = parseStyle(this.fillStyle)
      for (let row = y; row < y + height; row++) {
        for (let column = x; column < x + width; column++) {
          picture.blend(column, row, color)
        }
      }
    },
  }
  const canvas = document.createElement("canvas")
  canvas.width = t.widthDev
  canvas.height = t.heightDev
  Object.defineProperty(canvas, "getContext", { value: () => context })
  const renderer = createCanvas2DRenderer(canvas)
  renderer.setTheme(theme)
  renderer.beginFrame(t)
  for (const { batch, options } of scene) renderer.drawBatch(batch, options)
  renderer.endFrame()
  return picture
}

/** The largest difference of any channel of any pixel, in byte steps. */
function difference(scene: Scene, view: Viewport = viewport): number {
  const transform = deviceTransform(view)
  const gpu = drawOnGpu(scene, transform)
  const canvas = drawOnCanvas2D(scene, transform)
  let worst = 0
  for (let i = 0; i < gpu.data.length; i++) {
    worst = Math.max(worst, Math.abs(gpu.data[i] - canvas.data[i]))
  }
  return worst
}

// A style string carries its alpha to four decimals, which moves a channel
// by a small fraction of one step.
const ROUNDING = 0.1

function grid(view: Viewport = viewport): RectBatch {
  const batch = new RectBatch()
  writeGrid(batch, view, DEFAULT_TIME_GRID, pianoRows(), theme)
  return batch
}

const PINK = rgba(230, 60, 140)
const TEAL = rgba(40, 190, 170)
const DIM = rgba(125, 75, 25)

describe("Canvas 2D against the GPU renderers", () => {
  it("draws a see-through selected rect the same, border included", () => {
    const batch = new RectBatch()
    batch.push(1, 480, 960, 1, 1, PINK)
    batch.push(2, 960, 960, 1, 1, rgba(40, 190, 170, 128), RECT_SELECTED)
    batch.push(3, 2400, 720, 3, 1, rgba(230, 60, 140, 64), RECT_SELECTED)
    expect(difference([{ batch: grid() }, { batch }])).toBeLessThan(ROUNDING)
  })

  it("draws a see-through selection the same while it is dragged and resized", () => {
    const batch = new RectBatch()
    batch.push(1, 480, 960, 1, 1, PINK)
    batch.push(2, 960, 960, 1, 1, rgba(40, 190, 170, 128), RECT_SELECTED)
    batch.push(3, 1200, 480, 2, 1, TEAL)
    const drag = { dragTicks: 240, dragRows: 1 }
    expect(
      difference([{ batch: grid() }, { batch, options: drag }])
    ).toBeLessThan(ROUNDING)
    const stretch = { resizeEnd: 480, minLength: 240 }
    expect(
      difference([{ batch: grid() }, { batch, options: stretch }])
    ).toBeLessThan(ROUNDING)
  })

  it("stacks overlapping notes on a row in the same order", () => {
    const colors = [PINK, TEAL, DIM, TEAL, PINK, DIM]
    const batch = new RectBatch()
    // A pile on row 1, a chain on row 2 and loose notes on row 3.
    for (let i = 0; i < 6; i++)
      batch.push(i, 480 + i * 200, 960, 1, 1, colors[i])
    for (let i = 0; i < 6; i++) {
      batch.push(10 + i, 240 + i * 480, 600, 2, 1, colors[5 - i])
    }
    for (let i = 0; i < 6; i++) {
      batch.push(20 + i, i * 600, 480, 3, 1, colors[(i * 2) % 6])
    }
    expect(difference([{ batch: grid() }, { batch }])).toBeLessThan(ROUNDING)
  })

  it("stacks a pile deeper than it paints in groups in the same order", () => {
    const colors = [PINK, TEAL, DIM]
    const batch = new RectBatch()
    // Fourteen notes that all overlap, each a little later than the last,
    // then notes that lie on the top of the pile and on its foot.
    for (let i = 0; i < 14; i++) {
      batch.push(i, 240 + i * 60, 1200, 1, 1, colors[i % 3])
    }
    batch.push(20, 1900, 600, 1, 1, TEAL)
    batch.push(21, 2300, 600, 1, 1, rgba(230, 60, 140, 128))
    batch.push(22, 300, 2400, 0, 3, rgba(40, 190, 170, 96))
    expect(difference([{ batch: grid() }, { batch }])).toBeLessThan(ROUNDING)
  })

  it("draws crowded rows of solid notes the same, however they overlap", () => {
    const colors = [PINK, TEAL, DIM, rgba(250, 220, 40), rgba(90, 90, 240)]
    // A small generator with a fixed seed, so a failure can be repeated.
    let seed = 12345
    const random = (below: number) => {
      seed = (seed * 1103515245 + 12345) % 2147483648
      return Math.floor((seed / 2147483648) * below)
    }
    for (let round = 0; round < 20; round++) {
      const batch = new RectBatch()
      for (let i = 0; i < 120; i++) {
        const length = random(4) === 0 ? 600 + random(2400) : 30 + random(400)
        batch.push(
          i,
          random(3600),
          length,
          1 + random(4),
          1,
          colors[random(colors.length)],
          random(5) === 0 ? RECT_SELECTED : 0
        )
      }
      const items = indexBatch(batch).batch
      const options = round % 2 ? { dragTicks: 180, dragRows: -1 } : {}
      // A selected fill is mixed to a whole byte here and to a fraction on
      // a GPU, which is under half a step. A wrong order is many steps.
      expect(
        difference([{ batch: grid() }, { batch: items, options }])
      ).toBeLessThan(0.5)
    }
  })

  it("stacks a selection the same, over the notes it does not move", () => {
    const batch = new RectBatch()
    batch.push(1, 480, 1440, 1, 1, PINK)
    batch.push(2, 960, 960, 1, 1, TEAL, RECT_SELECTED)
    batch.push(3, 1200, 960, 1, 1, DIM, RECT_SELECTED)
    batch.push(4, 1680, 960, 1, 1, rgba(230, 60, 140, 128))
    batch.push(5, 1900, 960, 2, 2, TEAL)
    expect(
      difference([
        { batch: grid() },
        { batch, options: { dragTicks: -360, dragRows: 1 } },
      ])
    ).toBeLessThan(ROUNDING)
  })

  it("draws the grid alone the same, where its see-through lines cross", () => {
    expect(difference([{ batch: grid() }])).toBeLessThan(ROUNDING)
    const scrolled = { ...viewport, scrollTick: 1234.5, scrollRow: 60.3 }
    expect(difference([{ batch: grid(scrolled) }], scrolled)).toBeLessThan(
      ROUNDING
    )
  })

  it("draws slivers and thin rows the same, zoomed far out at 2x", () => {
    const far: Viewport = {
      ...viewport,
      dpr: 2,
      pxPerTick: 0.004,
      rowHeight: 5,
    }
    const batch = new RectBatch()
    for (let i = 0; i < 40; i++) {
      batch.push(
        i,
        i * 700,
        120 + (i % 5) * 400,
        1 + (i % 7),
        1,
        i % 3 ? PINK : DIM,
        i % 4 === 0 ? RECT_SELECTED : 0
      )
    }
    expect(difference([{ batch: grid(far) }, { batch }], far)).toBeLessThan(
      ROUNDING
    )
  })
})
