import type { Clip, ClipId, Pattern, PatternId } from "@/bindings"
import {
  BORDER_SHADE,
  deviceX,
  deviceY,
  mix,
  resizedSpan,
  rgbaToCss,
  rgbFromInt,
  visibleRange,
  visibleTicks,
  withAlpha,
  type GridTheme,
  type IndexedBatch,
  type OverlayFrame,
  type OverlayPainter,
  type Rgba,
} from "@/lib/canvas"

import { clipBodyColor, ORPHAN_COLOR } from "./clip-batch"
import { patternTicks, type NewClip } from "./edit"
import { loopPoints, previewOf, previewSpans, type ClipSpan } from "./preview"

/** How a drag in progress displaces the selected clips. */
export type DragState = {
  ticks: number
  rows: number
  resizeStart: number
  resizeEnd: number
  minLength: number
}

export const NO_DRAG: DragState = {
  ticks: 0,
  rows: 0,
  resizeStart: 0,
  resizeEnd: 0,
  minLength: 0,
}

/** What the painter draws from. Whoever owns it calls `invalidate` after a change. */
export interface PaintSource {
  readonly items: IndexedBatch | null
  readonly clipById: ReadonlyMap<ClipId, Clip>
  readonly patterns: ReadonlyMap<PatternId, Pattern>
  isMuted(clip: Clip): boolean
  readonly drag: DragState
  /** Clips that will exist once the button is released. */
  readonly ghosts: readonly NewClip[]
  /** Clips an erase or mute stroke has crossed. */
  readonly marked: ReadonlySet<ClipId>
  /** Show where the dragged clips came from, because they will stay there. */
  readonly cloneHint: boolean
  readonly songEnd: number
}

export type PaintStats = {
  /** Clips that got a name, a preview or both. */
  clips: number
  /** Note rects drawn for the previews. */
  notes: number
  /** Main-thread time of the last full repaint, in milliseconds. */
  ms: number
  /** Full repaints so far. A frame that only moves the playhead is not one. */
  repaints: number
}

// Sizes in CSS pixels. A clip smaller than these goes without that part.
const MIN_CLIP_WIDTH = 5
const MIN_LABEL_WIDTH = 22
const MIN_LABEL_HEIGHT = 12
const BAND_HEIGHT = 13
const MIN_HEIGHT_FOR_BAND = 25
const MIN_PREVIEW_HEIGHT = 6
const MIN_PASS_WIDTH = 10
const MAX_NOTE_HEIGHT = 4
const TEXT_PAD = 4
const FONT_SIZE = 10.5
const FIT_BUCKET = 4

/** Pixels of width a pass of the pattern needs per note before it is drawn. */
const NOTE_SPACING = 1.5
/**
 * Note rects per repaint. The spacing rule keeps a full screen well under
 * this; past it the remaining clips go without a preview.
 */
const NOTE_BUDGET = 20000
const DIVIDER_BUDGET = 1500

const DARK_INK = "rgba(0,0,0,0.84)"
const LIGHT_INK = "rgba(255,255,255,0.96)"

type Style = {
  body: string
  border: string
  band: string
  bandInk: string
  bodyInk: string
  note: string
}

type Sprite = {
  id: ClipId
  style: number
  pattern: Pattern | undefined
  span: ClipSpan
  x0: number
  x1: number
  y0: number
  y1: number
  ghost: boolean
}

function luminance(color: Rgba): number {
  return 0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b
}

function inkOn(color: Rgba): string {
  return luminance(color) > 150 ? DARK_INK : LIGHT_INK
}

/**
 * Draws what the rect renderer cannot: each clip's title bar, name and note
 * preview, the loop dividers, and the previews of a gesture in progress.
 *
 * The grid's overlay is repainted for every playhead move, 60 times a second
 * while the song plays. Names and previews do not change then, so they are
 * painted into a canvas of their own and that is copied into the overlay.
 * It is repainted only when the view, the clips or a gesture change.
 */
export class ClipPainter {
  readonly stats: PaintStats = { clips: 0, notes: 0, ms: 0, repaints: 0 }

  private readonly source: PaintSource
  private layer: HTMLCanvasElement | null = null
  private layerContext: CanvasRenderingContext2D | null = null
  private stale = true
  private key = ""
  private theme: GridTheme | null = null
  private styles = new Map<number, Style>()
  private stylePatterns: ReadonlyMap<PatternId, Pattern> | null = null
  private font = ""
  private widths = new Map<string, number>()
  private fits = new Map<string, string>()

  constructor(source: PaintSource) {
    this.source = source
  }

  /** Call after anything in the source changed. */
  invalidate(): void {
    this.stale = true
  }

  readonly paint: OverlayPainter = (ctx, frame) => {
    const { viewport, transform } = frame
    const key = `${viewport.scrollTick}|${viewport.scrollRow}|${viewport.pxPerTick}|${viewport.rowHeight}|${transform.widthDev}|${transform.heightDev}|${viewport.dpr}`
    const layer = this.layerFor(transform.widthDev, transform.heightDev)
    if (!layer || !this.layerContext) return
    if (this.stale || key !== this.key || frame.theme !== this.theme) {
      const started = performance.now()
      this.repaint(this.layerContext, frame)
      this.stats.ms = performance.now() - started
      this.stats.repaints++
      this.stale = false
      this.key = key
    }
    ctx.drawImage(layer, 0, 0)
  }

  private layerFor(width: number, height: number): HTMLCanvasElement | null {
    if (!this.layer) {
      this.layer = document.createElement("canvas")
      this.layerContext = this.layer.getContext("2d")
    }
    if (this.layer.width !== width || this.layer.height !== height) {
      this.layer.width = width
      this.layer.height = height
      this.stale = true
    }
    return this.layer
  }

  private styleFor(
    theme: GridTheme,
    pattern: Pattern | undefined,
    selected: boolean,
    muted: boolean
  ): number {
    const key =
      (pattern?.id ?? 0) * 4 + (selected ? 1 : 0) + (muted ? 2 : 0) + 4
    if (this.styles.has(key)) return key
    const color = pattern?.color ?? ORPHAN_COLOR
    const select = (fill: Rgba) =>
      selected ? mix(fill, theme.selectionFill, theme.selectionMix) : fill
    const body = select(clipBodyColor(theme, color, muted))
    const band = select(
      muted ? mix(theme.background, rgbFromInt(color), 0.4) : rgbFromInt(color)
    )
    const border = selected
      ? theme.selectionBorder
      : {
          r: Math.round(body.r * BORDER_SHADE),
          g: Math.round(body.g * BORDER_SHADE),
          b: Math.round(body.b * BORDER_SHADE),
          a: body.a,
        }
    const quiet = rgbaToCss(theme.mutedForeground)
    this.styles.set(key, {
      body: rgbaToCss(body),
      border: rgbaToCss(border),
      band: rgbaToCss(band),
      bandInk: muted ? quiet : inkOn(band),
      bodyInk: muted ? quiet : inkOn(body),
      note: rgbaToCss(mix(body, theme.foreground, muted ? 0.3 : 0.82)),
    })
    return key
  }

  private collect(frame: OverlayFrame): Sprite[] {
    const { viewport, transform, theme } = frame
    const { items, clipById, patterns, drag } = this.source
    const lw = transform.lineWidth
    const minWidth = MIN_CLIP_WIDTH * viewport.dpr
    const ticks = visibleTicks(viewport)
    const firstRow = Math.floor(viewport.scrollRow)
    const lastRow = Math.ceil(
      viewport.scrollRow + viewport.height / viewport.rowHeight
    )
    const sprites: Sprite[] = []

    const add = (
      id: ClipId,
      span: ClipSpan,
      row: number,
      pattern: Pattern | undefined,
      selected: boolean,
      muted: boolean,
      ghost: boolean
    ) => {
      if (row < firstRow || row >= lastRow) return
      if (span.start >= ticks.end || span.start + span.length <= ticks.start) {
        return
      }
      const x0 = deviceX(transform, span.start)
      const x1 = Math.max(x0 + lw, deviceX(transform, span.start + span.length))
      if (x1 - x0 < minWidth && !ghost && !this.source.marked.has(id)) return
      sprites.push({
        id,
        style: this.styleFor(theme, pattern, selected, muted),
        pattern,
        span,
        x0,
        x1,
        y0: deviceY(transform, row) + lw,
        y1: deviceY(transform, row + 1),
        ghost,
      })
    }

    if (items && items.batch.count > 0) {
      const batch = items.batch
      const moving =
        drag.ticks !== 0 ||
        drag.rows !== 0 ||
        drag.resizeStart !== 0 ||
        drag.resizeEnd !== 0
      // The same widening the grid does, so a clip dragged into view from
      // outside it gets its name too.
      const range = visibleRange(
        items,
        Math.min(
          ticks.start,
          ticks.start - drag.ticks - Math.max(0, drag.resizeEnd)
        ),
        Math.max(
          ticks.end,
          ticks.end - drag.ticks - Math.min(0, drag.resizeStart)
        )
      )
      for (let index = range.first; index < range.last; index++) {
        const id = batch.ids[index]
        const clip = clipById.get(id)
        if (!clip) continue
        const selected = batch.isSelected(index)
        let start = batch.start(index)
        let length = batch.length(index)
        let row = batch.row(index)
        let offset = clip.offset
        if (selected && moving) {
          const resized = resizedSpan(
            start,
            length,
            drag.resizeStart,
            drag.resizeEnd,
            drag.minLength
          )
          // Moving the start edge leaves the notes where they were.
          offset += resized.start - start
          start = resized.start + drag.ticks
          length = resized.length
          row += drag.rows
        }
        add(
          id,
          { start, length, offset },
          row,
          patterns.get(clip.content.pattern),
          selected,
          this.source.isMuted(clip),
          false
        )
      }
    }

    for (const ghost of this.source.ghosts) {
      add(
        -1,
        ghost,
        ghost.row,
        patterns.get(ghost.pattern),
        false,
        ghost.muted,
        true
      )
    }
    return sprites.sort((a, b) => a.style - b.style)
  }

  private repaint(ctx: CanvasRenderingContext2D, frame: OverlayFrame): void {
    const { viewport, transform, theme } = frame
    if (theme !== this.theme || this.source.patterns !== this.stylePatterns) {
      this.styles.clear()
      this.theme = theme
      this.stylePatterns = this.source.patterns
    }
    const dpr = viewport.dpr
    const lw = transform.lineWidth
    const width = transform.widthDev
    const height = transform.heightDev
    ctx.setTransform(1, 0, 0, 1, 0, 0)
    ctx.globalAlpha = 1
    ctx.clearRect(0, 0, width, height)

    const sprites = this.collect(frame)
    const styleOf = (sprite: Sprite) => this.styles.get(sprite.style)
    const bandHeight = Math.round(BAND_HEIGHT * dpr)
    const hasBand = (sprite: Sprite) =>
      sprite.y1 - sprite.y0 >= MIN_HEIGHT_FOR_BAND * dpr &&
      sprite.x1 - sprite.x0 > 2 * lw

    // Bodies of clips that do not exist yet. Existing ones are in the grid.
    ctx.globalAlpha = 0.78
    for (const sprite of sprites) {
      if (!sprite.ghost) continue
      const style = styleOf(sprite)
      if (!style) continue
      const w = sprite.x1 - sprite.x0
      const h = sprite.y1 - sprite.y0
      ctx.fillStyle = style.border
      ctx.fillRect(sprite.x0, sprite.y0, w, h)
      if (w > 2 * lw && h > 2 * lw) {
        ctx.fillStyle = style.body
        ctx.fillRect(sprite.x0 + lw, sprite.y0 + lw, w - 2 * lw, h - 2 * lw)
      }
    }
    ctx.globalAlpha = 1

    let current = -1
    for (const sprite of sprites) {
      if (!hasBand(sprite)) continue
      if (sprite.style !== current) {
        current = sprite.style
        ctx.fillStyle = styleOf(sprite)?.band ?? ""
      }
      ctx.fillRect(
        sprite.x0 + lw,
        sprite.y0 + lw,
        sprite.x1 - sprite.x0 - 2 * lw,
        Math.min(bandHeight, sprite.y1 - sprite.y0 - 2 * lw)
      )
    }

    const ticks = visibleTicks(viewport)
    const pad = Math.round(2 * dpr)
    const maxNote = Math.round(MAX_NOTE_HEIGHT * dpr)
    let notesLeft = NOTE_BUDGET
    current = -1
    for (const sprite of sprites) {
      if (notesLeft <= 0) break
      const pattern = sprite.pattern
      if (!pattern) continue
      const top = sprite.y0 + lw + (hasBand(sprite) ? bandHeight : 0) + pad
      const area = sprite.y1 - lw - pad - top
      if (area < MIN_PREVIEW_HEIGHT * dpr) continue
      const preview = previewOf(pattern)
      // A pass needs room for its notes, or the preview is only noise.
      const room = Math.max(MIN_PASS_WIDTH, preview.count * NOTE_SPACING) * dpr
      if (preview.length * transform.scaleX < room) continue
      if (sprite.style !== current) {
        current = sprite.style
        ctx.fillStyle = styleOf(sprite)?.note ?? ""
      }
      const left = sprite.x0 + lw
      const right = sprite.x1 - lw
      notesLeft -= previewSpans(
        preview,
        sprite.span,
        ticks.start,
        ticks.end,
        (start, end, noteTop, noteHeight) => {
          const x0 = Math.max(left, deviceX(transform, start))
          let x1 = Math.min(right, deviceX(transform, end))
          // A hairline between neighbours keeps a row of steps countable.
          if (x1 - x0 > 3 * lw) x1 -= lw
          if (x1 <= x0) x1 = x0 + lw
          if (x1 > right) return
          const unit = noteHeight * area
          // Lines too thin for a gap between them fill their whole share.
          const h =
            unit < 3 * lw
              ? Math.max(lw, Math.floor(unit))
              : Math.min(maxNote, Math.floor(unit) - lw)
          const y = top + Math.round(noteTop * area + (unit - h) / 2)
          ctx.fillRect(x0, y, x1 - x0, h)
        }
      )
    }

    let dividersLeft = DIVIDER_BUDGET
    ctx.fillStyle = rgbaToCss(withAlpha(theme.foreground, 0.22))
    for (const sprite of sprites) {
      if (dividersLeft <= 0) break
      const pattern = sprite.pattern
      if (!pattern) continue
      const pass = patternTicks(pattern)
      if (pass * transform.scaleX < MIN_PASS_WIDTH * dpr) continue
      for (const tick of loopPoints(
        sprite.span,
        pass,
        ticks.start,
        ticks.end
      )) {
        const x = deviceX(transform, tick)
        if (x <= sprite.x0 + lw || x >= sprite.x1 - lw) continue
        ctx.fillRect(x, sprite.y0 + lw, lw, sprite.y1 - sprite.y0 - 2 * lw)
        dividersLeft--
      }
    }

    this.setFont(ctx, dpr)
    ctx.textBaseline = "middle"
    const textPad = Math.round(TEXT_PAD * dpr)
    let named = 0
    let ink = ""
    for (const sprite of sprites) {
      const pattern = sprite.pattern
      const style = styleOf(sprite)
      if (!pattern || !style) continue
      const h = sprite.y1 - sprite.y0
      if (h < MIN_LABEL_HEIGHT * dpr) continue
      if (sprite.x1 - sprite.x0 < MIN_LABEL_WIDTH * dpr) continue
      // A clip that starts off-screen keeps its name at the left edge.
      const x = Math.max(sprite.x0, 0) + textPad
      const text = this.fit(ctx, pattern.name, sprite.x1 - textPad - x, dpr)
      if (text === "") continue
      const band = hasBand(sprite)
      const wanted = band ? style.bandInk : style.bodyInk
      if (wanted !== ink) {
        ink = wanted
        ctx.fillStyle = ink
      }
      const middle = band ? sprite.y0 + lw + bandHeight / 2 : sprite.y0 + h / 2
      ctx.fillText(text, x, Math.round(middle) + 0.5 * dpr)
      named++
    }

    const { marked } = this.source
    if (marked.size > 0) {
      ctx.fillStyle = rgbaToCss(withAlpha(theme.background, 0.66))
      for (const sprite of sprites) {
        if (!marked.has(sprite.id)) continue
        ctx.fillRect(
          sprite.x0,
          sprite.y0,
          sprite.x1 - sprite.x0,
          sprite.y1 - sprite.y0
        )
      }
    }

    if (this.source.cloneHint) this.paintOrigins(ctx, frame)
    this.paintSongEnd(ctx, frame)

    this.stats.clips = named
    this.stats.notes = NOTE_BUDGET - notesLeft
  }

  /** Outlines of the selected clips where they are stored, under a drag. */
  private paintOrigins(
    ctx: CanvasRenderingContext2D,
    frame: OverlayFrame
  ): void {
    const { viewport, transform, theme } = frame
    const items = this.source.items
    if (!items || items.batch.selectedCount === 0) return
    const lw = transform.lineWidth
    const ticks = visibleTicks(viewport)
    const range = visibleRange(items, ticks.start, ticks.end)
    ctx.fillStyle = rgbaToCss(withAlpha(theme.foreground, 0.55))
    for (let index = range.first; index < range.last; index++) {
      if (!items.batch.isSelected(index)) continue
      const x0 = deviceX(transform, items.batch.start(index))
      const x1 = Math.max(x0 + lw, deviceX(transform, items.batch.end(index)))
      const y0 = deviceY(transform, items.batch.row(index)) + lw
      const y1 = deviceY(transform, items.batch.row(index) + 1)
      ctx.fillRect(x0, y0, x1 - x0, lw)
      ctx.fillRect(x0, y1 - lw, x1 - x0, lw)
      ctx.fillRect(x0, y0, lw, y1 - y0)
      ctx.fillRect(x1 - lw, y0, lw, y1 - y0)
    }
  }

  /** A dashed line where the song ends. */
  private paintSongEnd(
    ctx: CanvasRenderingContext2D,
    frame: OverlayFrame
  ): void {
    const { viewport, transform, theme } = frame
    const end = this.source.songEnd
    if (end <= 0) return
    const x = deviceX(transform, end)
    if (x < 0 || x >= transform.widthDev) return
    const dash = Math.round(4 * viewport.dpr)
    ctx.fillStyle = rgbaToCss(withAlpha(theme.mutedForeground, 0.6))
    for (let y = 0; y < transform.heightDev; y += dash * 2) {
      ctx.fillRect(x, y, transform.lineWidth, dash)
    }
  }

  private setFont(ctx: CanvasRenderingContext2D, dpr: number): void {
    const font = `600 ${Math.round(FONT_SIZE * dpr * 10) / 10}px "Inter Variable", system-ui, sans-serif`
    if (font !== this.font) {
      this.font = font
      this.widths.clear()
      this.fits.clear()
    }
    ctx.font = font
  }

  /** `text`, cut short with an ellipsis when it is wider than `room`. */
  private fit(
    ctx: CanvasRenderingContext2D,
    text: string,
    room: number,
    dpr: number
  ): string {
    if (room <= 0) return ""
    let full = this.widths.get(text)
    if (full === undefined) {
      full = ctx.measureText(text).width
      this.widths.set(text, full)
    }
    if (full <= room) return text
    // Measuring is slow, so widths are rounded down to a few pixels and
    // each rounded width is worked out once per name.
    const bucket = Math.floor(room / (FIT_BUCKET * dpr))
    const key = `${bucket}\n${text}`
    const cached = this.fits.get(key)
    if (cached !== undefined) return cached
    const limit = bucket * FIT_BUCKET * dpr
    let low = 0
    let high = text.length - 1
    while (low < high) {
      const middle = Math.ceil((low + high) / 2)
      const candidate = `${text.slice(0, middle).trimEnd()}…`
      if (ctx.measureText(candidate).width <= limit) low = middle
      else high = middle - 1
    }
    // One or two letters and an ellipsis say nothing.
    const fitted = low >= 3 ? `${text.slice(0, low).trimEnd()}…` : ""
    if (this.fits.size > 4000) this.fits.clear()
    this.fits.set(key, fitted)
    return fitted
  }
}
