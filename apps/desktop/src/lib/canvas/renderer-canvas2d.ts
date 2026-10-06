import { mix, rgbaToCss, unpackRgba, type Rgba } from "./color"
import {
  GEOMETRY_STRIDE,
  RECT_FLAT,
  RECT_FULL_HEIGHT,
  RECT_FULL_WIDTH,
  RECT_HLINE,
  RECT_SELECTED,
  RECT_VLINE,
  type RectBatch,
} from "./rect-batch"
import {
  BORDER_SHADE,
  RendererUnavailableError,
  type DrawOptions,
  type RectRenderer,
  type RendererInfo,
} from "./renderer"
import type { GridTheme } from "./theme"
import type { DeviceTransform } from "./viewport"

const BLACK: Rgba = { r: 0, g: 0, b: 0, a: 255 }

// Setting fillStyle is the slow part of Canvas 2D, so rects are grouped by
// color and each color's style is set once per frame.
interface Palette {
  colorVersion: number
  colors: Uint8Array
  count: number
  slotOf: Uint32Array
  fill: string[]
  border: string[]
  selectedFill: string[]
}

const PASS_ALL = 0
const PASS_UNSELECTED = 1
const PASS_SELECTED = 2

class Canvas2DRenderer implements RectRenderer {
  readonly info: RendererInfo = {
    kind: "canvas2d",
    device: "Canvas 2D",
    gpuTiming: "none",
  }
  onRestored: (() => void) | null = null

  private readonly canvas: HTMLCanvasElement
  private readonly ctx: CanvasRenderingContext2D
  private theme: GridTheme | null = null
  private transform: DeviceTransform | null = null
  private palettes = new WeakMap<RectBatch, Palette>()
  private rects = new Int32Array(1024 * 4)
  private slots = new Uint32Array(1024)
  private bordered = new Uint8Array(1024)
  private order = new Uint32Array(1024)
  private counts = new Uint32Array(64)

  constructor(canvas: HTMLCanvasElement) {
    const ctx = canvas.getContext("2d", { alpha: false })
    if (!ctx) throw new RendererUnavailableError("canvas2d", "no 2d context")
    this.canvas = canvas
    this.ctx = ctx
  }

  resize(widthDev: number, heightDev: number): void {
    if (this.canvas.width !== widthDev) this.canvas.width = widthDev
    if (this.canvas.height !== heightDev) this.canvas.height = heightDev
  }

  setTheme(theme: GridTheme): void {
    this.theme = theme
    // Selected fills depend on the theme.
    this.palettes = new WeakMap()
  }

  beginFrame(transform: DeviceTransform): void {
    this.transform = transform
    const ctx = this.ctx
    ctx.setTransform(1, 0, 0, 1, 0, 0)
    ctx.fillStyle = rgbaToCss(this.theme?.background ?? BLACK)
    ctx.fillRect(0, 0, this.canvas.width, this.canvas.height)
  }

  drawBatch(batch: RectBatch, options: DrawOptions = {}): void {
    const first = Math.max(0, options.first ?? 0)
    const last = Math.min(batch.count, options.last ?? batch.count)
    if (last <= first || !this.transform || !this.theme) return
    const palette = this.paletteFor(batch, this.theme)
    const dragTicks = options.dragTicks ?? 0
    const dragRows = options.dragRows ?? 0
    if (batch.selectedCount === 0) {
      this.drawPass(batch, palette, first, last, PASS_ALL, 0, 0)
      return
    }
    this.drawPass(batch, palette, first, last, PASS_UNSELECTED, 0, 0)
    this.drawPass(
      batch,
      palette,
      first,
      last,
      PASS_SELECTED,
      dragTicks,
      dragRows
    )
  }

  endFrame(): void {
    this.transform = null
  }

  // Canvas 2D raster runs in the GPU process and the only way to wait for
  // it is a pixel readback. Repeated readbacks can make Chromium move the
  // canvas to the CPU, so a measurement must not use them.
  finish(): boolean {
    return false
  }

  release(batch: RectBatch): void {
    this.palettes.delete(batch)
  }

  setGpuTiming(): boolean {
    return false
  }

  takeGpuTimes(): number[] {
    return []
  }

  dispose(): void {
    this.palettes = new WeakMap()
  }

  private paletteFor(batch: RectBatch, theme: GridTheme): Palette {
    const cached = this.palettes.get(batch)
    if (
      cached &&
      cached.colorVersion === batch.colorVersion &&
      cached.colors === batch.colors &&
      cached.count === batch.count
    ) {
      return cached
    }
    const count = batch.count
    const colors = batch.colors
    const slotOf =
      cached && cached.slotOf.length >= count
        ? cached.slotOf
        : new Uint32Array(Math.max(count, 16))
    const slotByColor = new Map<number, number>()
    const fill: string[] = []
    const border: string[] = []
    const selectedFill: string[] = []
    for (let i = 0; i < count; i++) {
      const c = i * 4
      const key =
        ((colors[c] << 24) |
          (colors[c + 1] << 16) |
          (colors[c + 2] << 8) |
          colors[c + 3]) >>>
        0
      let slot = slotByColor.get(key)
      if (slot === undefined) {
        slot = fill.length
        slotByColor.set(key, slot)
        const color = unpackRgba(key)
        fill.push(rgbaToCss(color))
        border.push(rgbaToCss(shade(color)))
        selectedFill.push(
          rgbaToCss({
            ...mix(color, theme.selectionFill, theme.selectionMix),
            a: color.a,
          })
        )
      }
      slotOf[i] = slot
    }
    const palette: Palette = {
      colorVersion: batch.colorVersion,
      colors,
      count,
      slotOf,
      fill,
      border,
      selectedFill,
    }
    this.palettes.set(batch, palette)
    return palette
  }

  private reserve(rectCount: number, slotCount: number): void {
    if (this.slots.length < rectCount) {
      const capacity = Math.max(rectCount, this.slots.length * 2)
      this.rects = new Int32Array(capacity * 4)
      this.slots = new Uint32Array(capacity)
      this.bordered = new Uint8Array(capacity)
      this.order = new Uint32Array(capacity)
    }
    if (this.counts.length < slotCount + 1) {
      this.counts = new Uint32Array((slotCount + 1) * 2)
    }
  }

  private drawPass(
    batch: RectBatch,
    palette: Palette,
    first: number,
    last: number,
    pass: number,
    dragTicks: number,
    dragRows: number
  ): void {
    const t = this.transform
    const theme = this.theme
    if (!t || !theme) return
    const slotCount = palette.fill.length
    this.reserve(last - first, slotCount)
    const { rects, slots, bordered, order, counts } = this
    const geometry = batch.geometry
    const flags = batch.flags
    const slotOf = palette.slotOf
    const width = this.canvas.width
    const height = this.canvas.height
    const lw = t.lineWidth
    const { scrollTick, scaleX, offsetX, scaleY, offsetY } = t

    counts.fill(0, 0, slotCount + 1)
    let n = 0
    for (let i = first; i < last; i++) {
      const f = flags[i]
      const selected = (f & RECT_SELECTED) !== 0
      if (pass === PASS_UNSELECTED && selected) continue
      if (pass === PASS_SELECTED && !selected) continue
      const g = i * GEOMETRY_STRIDE
      let start = geometry[g]
      let row = geometry[g + 2]
      if (selected) {
        start += dragTicks
        row += dragRows
      }
      const flat = (f & RECT_FLAT) !== 0

      // Same order of operations as the shaders, so every renderer puts a
      // rect on the same pixels.
      let x0 = Math.floor((start - scrollTick) * scaleX - offsetX + 0.5)
      let x1 = Math.floor(
        (start + geometry[g + 1] - scrollTick) * scaleX - offsetX + 0.5
      )
      let y0 = Math.floor(row * scaleY - offsetY + 0.5)
      let y1 = Math.floor((row + geometry[g + 3]) * scaleY - offsetY + 0.5)
      // The row line above an item stays visible.
      if (!flat) y0 += lw
      if (f & RECT_FULL_WIDTH) {
        x0 = 0
        x1 = width
      }
      if (f & RECT_FULL_HEIGHT) {
        y0 = 0
        y1 = height
      }
      if (f & RECT_VLINE) x1 = x0 + lw
      if (f & RECT_HLINE) y1 = y0 + lw
      if (x1 < x0 + lw) x1 = x0 + lw
      if (y1 < y0 + lw) y1 = y0 + lw
      if (x1 <= 0 || x0 >= width || y1 <= 0 || y0 >= height) continue

      const w = x1 - x0
      const h = y1 - y0
      const r = n * 4
      rects[r] = x0
      rects[r + 1] = y0
      rects[r + 2] = w
      rects[r + 3] = h
      const slot = slotOf[i]
      slots[n] = slot
      bordered[n] = !flat && w >= 3 * lw && h >= 3 * lw ? 1 : 0
      counts[slot + 1]++
      n++
    }
    if (n === 0) return

    // Counting sort of the visible rects by color slot.
    for (let s = 0; s < slotCount; s++) counts[s + 1] += counts[s]
    for (let k = 0; k < n; k++) order[counts[slots[k]]++] = k

    const ctx = this.ctx
    const selectedPass = pass === PASS_SELECTED
    const selectionBorder = rgbaToCss(theme.selectionBorder)
    let begin = 0
    for (let s = 0; s < slotCount; s++) {
      // After the placement loop, counts[s] is the end of slot s.
      const end = counts[s]
      if (end === begin) continue

      let anyBorder = false
      for (let o = begin; o < end; o++) {
        if (bordered[order[o]]) {
          anyBorder = true
          break
        }
      }
      if (anyBorder) {
        ctx.fillStyle = selectedPass ? selectionBorder : palette.border[s]
        for (let o = begin; o < end; o++) {
          const k = order[o]
          if (!bordered[k]) continue
          const r = k * 4
          ctx.fillRect(rects[r], rects[r + 1], rects[r + 2], rects[r + 3])
        }
      }

      ctx.fillStyle = selectedPass ? palette.selectedFill[s] : palette.fill[s]
      for (let o = begin; o < end; o++) {
        const k = order[o]
        const r = k * 4
        if (bordered[k]) {
          ctx.fillRect(
            rects[r] + lw,
            rects[r + 1] + lw,
            rects[r + 2] - 2 * lw,
            rects[r + 3] - 2 * lw
          )
        } else {
          ctx.fillRect(rects[r], rects[r + 1], rects[r + 2], rects[r + 3])
        }
      }
      begin = end
    }
  }
}

function shade(color: Rgba): Rgba {
  return {
    r: Math.round(color.r * BORDER_SHADE),
    g: Math.round(color.g * BORDER_SHADE),
    b: Math.round(color.b * BORDER_SHADE),
    a: color.a,
  }
}

export function createCanvas2DRenderer(
  canvas: HTMLCanvasElement
): RectRenderer {
  return new Canvas2DRenderer(canvas)
}
