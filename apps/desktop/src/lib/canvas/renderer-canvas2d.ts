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
  RendererUnavailableError,
  resizedSpan,
  shadeChannel,
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
  selectedBorder: string[]
  /** 1 where the color has no transparency. */
  opaque: Uint8Array
}

const PASS_ALL = 0
const PASS_UNSELECTED = 1
const PASS_SELECTED = 2

/**
 * How many layers `paintLayered` paints grouped by color. A rect that needs
 * a layer past these is painted on its own, after the groups.
 */
const LAYERS = 8
const NOT_REACHED = -2147483648

class Canvas2DRenderer implements RectRenderer {
  readonly info: RendererInfo = {
    kind: "canvas2d",
    device: "Canvas 2D",
    gpuTiming: "none",
  }
  onRestored: (() => void) | null = null
  onLost: (() => void) | null = null

  private readonly canvas: HTMLCanvasElement
  private readonly ctx: CanvasRenderingContext2D
  private theme: GridTheme | null = null
  private transform: DeviceTransform | null = null
  private palettes = new WeakMap<RectBatch, Palette>()
  // Scratch space for one pass, grown as needed and never shrunk.
  private rects = new Int32Array(1024 * 4)
  private slots = new Uint32Array(1024)
  private bordered = new Uint8Array(1024)
  private rowFrom = new Int32Array(1024)
  private rowTo = new Int32Array(1024)
  private layers = new Uint8Array(1024)
  private order = new Uint32Array(2048)
  private pieceOf = new Uint32Array(2048)
  private pieceX = new Int32Array(2048)
  private pieceWidth = new Int32Array(2048)
  private coverEnd = new Int32Array(1024)
  private coverNext = new Int32Array(1024)
  private counts = new Uint32Array(64)
  private rowEnds = new Int32Array(256 * LAYERS)
  private rowDepth = new Uint8Array(256)
  private rowMark = new Int32Array(256)

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
      this.drawPass(batch, palette, first, last, PASS_ALL, 0, 0, options)
      return
    }
    this.drawPass(batch, palette, first, last, PASS_UNSELECTED, 0, 0, options)
    this.drawPass(
      batch,
      palette,
      first,
      last,
      PASS_SELECTED,
      dragTicks,
      dragRows,
      options
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
    const selectedBorder: string[] = []
    const opaque: number[] = []
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
        // The selection border is as see-through as the rect it frames.
        selectedBorder.push(rgbaToCss({ ...theme.selectionBorder, a: color.a }))
        opaque.push(color.a >= 255 ? 1 : 0)
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
      selectedBorder,
      opaque: Uint8Array.from(opaque),
    }
    this.palettes.set(batch, palette)
    return palette
  }

  private reserve(rectCount: number, slotCount: number, rows: number): void {
    if (this.slots.length < rectCount) {
      const capacity = Math.max(rectCount, this.slots.length * 2)
      this.rects = new Int32Array(capacity * 4)
      this.slots = new Uint32Array(capacity)
      this.bordered = new Uint8Array(capacity)
      this.rowFrom = new Int32Array(capacity)
      this.rowTo = new Int32Array(capacity)
      this.layers = new Uint8Array(capacity)
      // A rect is painted in at most two pieces on average.
      this.order = new Uint32Array(capacity * 2)
      this.pieceOf = new Uint32Array(capacity * 2)
      this.pieceX = new Int32Array(capacity * 2)
      this.pieceWidth = new Int32Array(capacity * 2)
      this.coverEnd = new Int32Array(capacity)
      this.coverNext = new Int32Array(capacity)
    }
    if (this.counts.length < slotCount * LAYERS + 1) {
      this.counts = new Uint32Array((slotCount * LAYERS + 1) * 2)
    }
    if (this.rowDepth.length < rows) {
      this.rowEnds = new Int32Array(rows * 2 * LAYERS)
      this.rowDepth = new Uint8Array(rows * 2)
      this.rowMark = new Int32Array(rows * 2)
    }
  }

  /*
   * The GPU renderers paint a batch in index order, so where two rects
   * overlap the later one is on top. Painting in that order here would set
   * a fill style per rect, which is the slow part of Canvas 2D. So a pass
   * first works out where every rect lands, then paints the same picture
   * in an order that sets each style once: `paintVisible` when it can,
   * `paintLayered` for the rest.
   */
  private drawPass(
    batch: RectBatch,
    palette: Palette,
    first: number,
    last: number,
    pass: number,
    dragTicks: number,
    dragRows: number,
    resize: DrawOptions
  ): void {
    const t = this.transform
    if (!t) return
    const slotCount = palette.fill.length
    const { scrollTick, scaleX, offsetX, scaleY, offsetY } = t
    // One slot per row on screen and one for a row that just reaches in
    // from above.
    const firstRow = Math.floor(offsetY / scaleY) - 1
    const rowCount = Math.ceil(this.canvas.height / scaleY) + 3
    this.reserve(last - first, slotCount, rowCount)
    const { rects, slots, bordered, rowFrom, rowTo, rowMark } = this
    const geometry = batch.geometry
    const flags = batch.flags
    const slotOf = palette.slotOf
    const opaque = palette.opaque
    const width = this.canvas.width
    const height = this.canvas.height
    const lw = t.lineWidth
    const resizeStart = resize.resizeStart ?? 0
    const resizeEnd = resize.resizeEnd ?? 0
    const minLength = resize.minLength ?? 0
    const resizing = resizeStart !== 0 || resizeEnd !== 0

    // Whether every rect is a solid item on one row, with the rects of a
    // row coming in order of their left edge. Notes and clips are.
    let plain = true
    rowMark.fill(NOT_REACHED, 0, rowCount)
    let n = 0
    for (let i = first; i < last; i++) {
      const f = flags[i]
      const selected = (f & RECT_SELECTED) !== 0
      if (pass === PASS_UNSELECTED && selected) continue
      if (pass === PASS_SELECTED && !selected) continue
      const g = i * GEOMETRY_STRIDE
      let start = geometry[g]
      let length = geometry[g + 1]
      let row = geometry[g + 2]
      if (selected) {
        if (resizing) {
          const span = resizedSpan(
            start,
            length,
            resizeStart,
            resizeEnd,
            minLength
          )
          start = span.start
          length = span.length
        }
        start += dragTicks
        row += dragRows
      }
      const flat = (f & RECT_FLAT) !== 0

      // Same order of operations as the shaders, so every renderer puts a
      // rect on the same pixels.
      let x0 = Math.floor((start - scrollTick) * scaleX - offsetX + 0.5)
      let x1 = Math.floor(
        (start + length - scrollTick) * scaleX - offsetX + 0.5
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

      const slot = slotOf[i]
      let r0 = row - firstRow
      let r1 = r0 + Math.max(1, geometry[g + 3])
      if (f & RECT_FULL_HEIGHT) {
        r0 = 0
        r1 = rowCount
      }
      if (
        flat ||
        opaque[slot] === 0 ||
        r1 !== r0 + 1 ||
        r0 < 0 ||
        r0 >= rowCount ||
        x0 < rowMark[r0]
      ) {
        plain = false
      } else {
        rowMark[r0] = x0
      }
      // Rows off the screen share the slot at that end. Only `paintLayered`
      // gets to see those, and for it too many rows in one slot is safe.
      r0 = r0 < 0 ? 0 : r0 >= rowCount ? rowCount - 1 : r0
      r1 = r1 <= r0 ? r0 + 1 : r1 > rowCount ? rowCount : r1

      const w = x1 - x0
      const h = y1 - y0
      const r = n * 4
      rects[r] = x0
      rects[r + 1] = y0
      rects[r + 2] = w
      rects[r + 3] = h
      slots[n] = slot
      bordered[n] = !flat && w >= 3 * lw && h >= 3 * lw ? 1 : 0
      rowFrom[n] = r0
      rowTo[n] = r1
      n++
    }
    if (n === 0) return

    const selectedPass = pass === PASS_SELECTED
    const fills = selectedPass ? palette.selectedFill : palette.fill
    const borders = selectedPass ? palette.selectedBorder : palette.border
    if (plain) this.paintVisible(n, slotCount, rowCount, fills, borders, lw)
    else this.paintLayered(n, slotCount, rowCount, fills, borders, opaque, lw)
  }

  /*
   * For solid rects. What shows of a rect is the part no later rect of its
   * row covers, and those parts never overlap, so they can be painted in
   * any order: grouped by color, however deep the rects pile up. Going
   * through the rects backwards, each row keeps the stretches covered so
   * far as a list that starts with the leftmost one.
   */
  private paintVisible(
    n: number,
    slotCount: number,
    rowCount: number,
    fills: readonly string[],
    borders: readonly string[],
    lw: number
  ): void {
    const { rects, slots, bordered, rowFrom, order, counts } = this
    const { pieceOf, pieceX, pieceWidth, coverEnd, coverNext } = this
    // Per row, the rect whose left edge starts the leftmost covered stretch.
    const rowHead = this.rowMark
    rowHead.fill(-1, 0, rowCount)
    counts.fill(0, 0, slotCount + 1)
    let pieces = 0
    for (let k = n - 1; k >= 0; k--) {
      const x0 = rects[k * 4]
      const x1 = x0 + rects[k * 4 + 2]
      const row = rowFrom[k]
      let shown = x0
      let coveredTo = x1
      let stretch = rowHead[row]
      // Every stretch starts at or right of x0, as the rects of a row come
      // in order of their left edge.
      while (stretch !== -1 && rects[stretch * 4] <= x1) {
        const from = rects[stretch * 4]
        if (from > shown) {
          pieceOf[pieces] = k
          pieceX[pieces] = shown
          pieceWidth[pieces] = from - shown
          pieces++
          counts[slots[k] + 1]++
        }
        const to = coverEnd[stretch]
        if (to > shown) shown = to
        if (to > coveredTo) coveredTo = to
        stretch = coverNext[stretch]
      }
      if (shown < x1) {
        pieceOf[pieces] = k
        pieceX[pieces] = shown
        pieceWidth[pieces] = x1 - shown
        pieces++
        counts[slots[k] + 1]++
      }
      // This rect and the stretches it reached become one stretch.
      coverEnd[k] = coveredTo
      coverNext[k] = stretch
      rowHead[row] = k
    }

    for (let s = 0; s < slotCount; s++) counts[s + 1] += counts[s]
    for (let p = 0; p < pieces; p++) order[counts[slots[pieceOf[p]]]++] = p

    const ctx = this.ctx
    let begin = 0
    for (let s = 0; s < slotCount; s++) {
      // After the placement loop, counts[s] is the end of slot s.
      const end = counts[s]
      if (end === begin) continue

      let anyBorder = false
      for (let o = begin; o < end; o++) {
        if (bordered[pieceOf[order[o]]]) {
          anyBorder = true
          break
        }
      }
      if (anyBorder) {
        ctx.fillStyle = borders[s]
        for (let o = begin; o < end; o++) {
          const p = order[o]
          const r = pieceOf[p] * 4
          if (!bordered[pieceOf[p]]) continue
          // The fill that follows covers the middle.
          ctx.fillRect(pieceX[p], rects[r + 1], pieceWidth[p], rects[r + 3])
        }
      }

      ctx.fillStyle = fills[s]
      for (let o = begin; o < end; o++) {
        const p = order[o]
        const r = pieceOf[p] * 4
        if (!bordered[pieceOf[p]]) {
          ctx.fillRect(pieceX[p], rects[r + 1], pieceWidth[p], rects[r + 3])
          continue
        }
        // The rect's fill without its border, as far as this piece goes.
        const left = Math.max(pieceX[p], rects[r] + lw)
        const right = Math.min(
          pieceX[p] + pieceWidth[p],
          rects[r] + rects[r + 2] - lw
        )
        if (right > left) {
          ctx.fillRect(
            left,
            rects[r + 1] + lw,
            right - left,
            rects[r + 3] - 2 * lw
          )
        }
      }
      begin = end
    }
  }

  /*
   * For everything else: grid lines, see-through rects, rects over several
   * rows. Every rect gets a layer, one more than the highest layer among
   * the earlier rects it reaches into. Rects of one layer never overlap, so
   * a layer is painted grouped by color, and the layers one after the other
   * give the same picture as index order.
   */
  private paintLayered(
    n: number,
    slotCount: number,
    rowCount: number,
    fills: readonly string[],
    borders: readonly string[],
    opaque: Uint8Array,
    lw: number
  ): void {
    const { rects, slots, bordered, rowFrom, rowTo, layers } = this
    const { order, counts, rowEnds, rowDepth } = this
    rowDepth.fill(0, 0, rowCount)
    counts.fill(0, 0, slotCount * LAYERS + 1)
    let topLayer = 0
    let deepCount = 0
    for (let k = 0; k < n; k++) {
      const x0 = rects[k * 4]
      const x1 = x0 + rects[k * 4 + 2]
      const r0 = rowFrom[k]
      const r1 = rowTo[k]
      // An earlier rect of these rows that ends right of this one's start
      // may lie under it. Taking one that does not only costs a layer.
      let layer = 0
      for (let at = r0; at < r1; at++) {
        const base = at * LAYERS
        for (let l = rowDepth[at] - 1; l >= layer; l--) {
          if (rowEnds[base + l] > x0) {
            layer = l + 1
            break
          }
        }
      }
      // The rects past the last layer share its record, so whatever lies
      // on one of them is found to be past the last layer too.
      const record = layer < LAYERS ? layer : LAYERS - 1
      for (let at = r0; at < r1; at++) {
        const base = at * LAYERS
        for (let l = rowDepth[at]; l <= record; l++) {
          rowEnds[base + l] = NOT_REACHED
        }
        if (rowDepth[at] <= record) rowDepth[at] = record + 1
        if (x1 > rowEnds[base + record]) rowEnds[base + record] = x1
      }
      layers[k] = layer
      if (layer < LAYERS) {
        counts[layer * slotCount + slots[k] + 1]++
        if (layer > topLayer) topLayer = layer
      } else {
        deepCount++
      }
    }

    // Counting sort by layer, then by color slot.
    const groupCount = (topLayer + 1) * slotCount
    for (let key = 0; key < groupCount; key++) counts[key + 1] += counts[key]
    for (let k = 0; k < n; k++) {
      if (layers[k] < LAYERS) {
        order[counts[layers[k] * slotCount + slots[k]]++] = k
      }
    }

    const ctx = this.ctx
    let begin = 0
    for (let key = 0; key < groupCount; key++) {
      // After the placement loop, counts[key] is the end of the group.
      const end = counts[key]
      if (end === begin) continue
      const s = key % slotCount

      let anyBorder = false
      for (let o = begin; o < end; o++) {
        if (bordered[order[o]]) {
          anyBorder = true
          break
        }
      }
      if (anyBorder) {
        ctx.fillStyle = borders[s]
        const solid = opaque[s] === 1
        for (let o = begin; o < end; o++) {
          const k = order[o]
          if (!bordered[k]) continue
          const r = k * 4
          if (solid) {
            // The fill that follows covers the middle.
            ctx.fillRect(rects[r], rects[r + 1], rects[r + 2], rects[r + 3])
          } else {
            frame(ctx, rects[r], rects[r + 1], rects[r + 2], rects[r + 3], lw)
          }
        }
      }

      ctx.fillStyle = fills[s]
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
    if (deepCount === 0) return

    // The rects past the last layer, in batch order, each one whole.
    let style = ""
    for (let k = 0; k < n; k++) {
      if (layers[k] < LAYERS) continue
      const s = slots[k]
      const r = k * 4
      if (bordered[k]) {
        if (style !== borders[s]) ctx.fillStyle = style = borders[s]
        if (opaque[s] === 1) {
          ctx.fillRect(rects[r], rects[r + 1], rects[r + 2], rects[r + 3])
        } else {
          frame(ctx, rects[r], rects[r + 1], rects[r + 2], rects[r + 3], lw)
        }
        if (style !== fills[s]) ctx.fillStyle = style = fills[s]
        ctx.fillRect(
          rects[r] + lw,
          rects[r + 1] + lw,
          rects[r + 2] - 2 * lw,
          rects[r + 3] - 2 * lw
        )
      } else {
        if (style !== fills[s]) ctx.fillStyle = style = fills[s]
        ctx.fillRect(rects[r], rects[r + 1], rects[r + 2], rects[r + 3])
      }
    }
  }
}

/**
 * The border of a see-through rect: its four sides, each pixel painted
 * once. A filled rect under the fill would show through it.
 */
function frame(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  lw: number
): void {
  ctx.fillRect(x, y, w, lw)
  ctx.fillRect(x, y + h - lw, w, lw)
  ctx.fillRect(x, y + lw, lw, h - 2 * lw)
  ctx.fillRect(x + w - lw, y + lw, lw, h - 2 * lw)
}

function shade(color: Rgba): Rgba {
  return {
    r: shadeChannel(color.r),
    g: shadeChannel(color.g),
    b: shadeChannel(color.b),
    a: color.a,
  }
}

export function createCanvas2DRenderer(
  canvas: HTMLCanvasElement
): RectRenderer {
  return new Canvas2DRenderer(canvas)
}
