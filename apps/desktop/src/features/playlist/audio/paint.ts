import { gainToFaderPosition } from "@/components/audio"
import { deviceX, rgbaToCss, withAlpha, type OverlayFrame } from "@/lib/canvas"

import type { Box } from "../clip-box"
import { tickAtDeviceX, type ClipSprite, type ClipStyle } from "../sprite"
import {
  audioEndTick,
  fadeGainAt,
  fadeInGain,
  fadeOutGain,
  filePosition,
  type AudioTiming,
} from "./geometry"
import {
  FADE_HANDLE,
  fadeHandleLeft,
  fadeHandleTop,
  gainHandleBox,
  hasFadeHandles,
  hasGainHandle,
} from "./handles"
import { peakRange, type SamplePeaks } from "./peaks"

type AudioContent = Extract<ClipSprite["content"], { type: "audio" }>

/** A waveform is magnified so its loudest peak fills the clip, up to this. */
const MAX_DISPLAY_GAIN = 100
/** Line segments a fade curve is drawn with. */
const FADE_STEPS = 12
/** A waveform narrower than this many CSS pixels is not worth drawing. */
const MIN_WAVE_WIDTH = 6
const MIN_WAVE_HEIGHT = 6

export type AudioPaint = {
  /** The sample's waveform, or null when the sample is not in the project. */
  peaks: SamplePeaks | null
  tempoBpm: number
  /** Where the content goes: the clip under its title bar, in device pixels. */
  area: Box
  /** The clip's whole rect in CSS pixels, which the handles are laid out by. */
  box: Box
  /** Columns of waveform that may still be drawn this repaint. */
  budget: { columns: number }
}

function timingOf(sprite: ClipSprite, content: AudioContent): AudioTiming {
  return {
    start: sprite.span.start,
    length: sprite.span.length,
    offset: sprite.span.offset,
    pitch: content.pitch,
    stretch: content.stretch,
    reverse: content.reverse,
  }
}

/**
 * Draws the waveform of an audio clip: the part of the file that plays
 * under each pixel, at the height the clip's gain and fades give it.
 * Returns the device x where the sound stops inside the clip.
 */
function paintWave(
  ctx: CanvasRenderingContext2D,
  frame: OverlayFrame,
  sprite: ClipSprite,
  content: AudioContent,
  paint: AudioPaint,
  durationSecs: number,
  pyramid: Extract<SamplePeaks, { status: "ready" }>["pyramid"]
): void {
  const { transform, viewport } = frame
  const { area, tempoBpm } = paint
  const dpr = viewport.dpr
  const lw = transform.lineWidth
  const xa = Math.max(area.left, 0)
  const xb = Math.min(area.right, transform.widthDev)
  const height = area.bottom - area.top
  if (xb - xa < MIN_WAVE_WIDTH * dpr || height < MIN_WAVE_HEIGHT * dpr) return
  if (paint.budget.columns <= 0) return
  paint.budget.columns -= xb - xa

  const timing = timingOf(sprite, content)
  const center = (area.top + area.bottom) / 2
  const norm = Math.min(MAX_DISPLAY_GAIN, 1 / Math.max(pyramid.peak, 1e-6))
  const reach = (height / 2) * gainToFaderPosition(content.gain) * norm
  const fades = {
    start: timing.start,
    length: timing.length,
    fadeIn: content.fadeIn,
    fadeOut: content.fadeOut,
  }
  const faded = content.fadeIn > 0 || content.fadeOut > 0

  // The place in the file moves in a straight line across the clip.
  const tickA = tickAtDeviceX(transform, xa)
  const perColumn = 1 / transform.scaleX
  const fileA = filePosition(timing, tickA, durationSecs, tempoBpm)
  const filePerColumn =
    filePosition(timing, tickA + perColumn, durationSecs, tempoBpm) - fileA

  ctx.beginPath()
  for (let x = xa; x < xb; x += 1) {
    const from = fileA + (x - xa) * filePerColumn
    const to = from + filePerColumn
    if (Math.max(from, to) <= 0 || Math.min(from, to) >= 1) continue
    const [min, max] = peakRange(pyramid, from, to)
    const level = faded
      ? reach * fadeGainAt(fades, tickA + (x - xa + 0.5) * perColumn)
      : reach
    const top = Math.max(area.top, Math.round(center - max * level))
    const bottom = Math.min(area.bottom, Math.round(center - min * level))
    ctx.rect(x, top, 1, Math.max(lw, bottom - top))
  }
  ctx.fill()
}

/**
 * One half of the shape of a fade: the level from silence to full, as a
 * line from the middle of the clip out to its top (`side` -1) or its
 * bottom (`side` 1). The two halves are the outline the waveform keeps
 * inside while it fades.
 */
function fadePath(
  ctx: CanvasRenderingContext2D,
  edge: "in" | "out",
  x0: number,
  x1: number,
  area: Box,
  side: -1 | 1,
  start: "move" | "line"
) {
  const half = (area.bottom - area.top) / 2
  const center = area.top + half
  for (let step = 0; step <= FADE_STEPS; step += 1) {
    const part = step / FADE_STEPS
    const gain = edge === "in" ? fadeInGain(part) : fadeOutGain(part)
    const x = x0 + (x1 - x0) * part
    const y = center + side * half * gain
    if (step === 0 && start === "move") ctx.moveTo(x, y)
    else ctx.lineTo(x, y)
  }
}

/**
 * Draws what is particular to an audio clip over its body: the waveform,
 * a dimmed stretch where the audio has run out before the clip ends, the
 * fades as curves with their handles, and the gain handle.
 */
export function paintAudio(
  ctx: CanvasRenderingContext2D,
  frame: OverlayFrame,
  sprite: ClipSprite,
  style: ClipStyle,
  paint: AudioPaint
): void {
  const content = sprite.content
  if (content.type !== "audio") return
  const { transform, viewport, theme } = frame
  const { area, peaks, tempoBpm } = paint
  const dpr = viewport.dpr
  const lw = transform.lineWidth
  if (area.right <= area.left || area.bottom <= area.top) return

  if (peaks?.status === "ready") {
    ctx.fillStyle = style.note
    paintWave(
      ctx,
      frame,
      sprite,
      content,
      paint,
      peaks.durationSecs,
      peaks.pyramid
    )
    // Where the audio ends before the clip does, the rest plays nothing.
    const end = audioEndTick(
      timingOf(sprite, content),
      peaks.durationSecs,
      tempoBpm
    )
    const xEnd = Math.max(area.left, deviceX(transform, end))
    if (xEnd < area.right - lw) {
      ctx.fillStyle = rgbaToCss(withAlpha(theme.background, 0.58))
      ctx.fillRect(xEnd, area.top, area.right - xEnd, area.bottom - area.top)
      ctx.fillStyle = style.note
      ctx.fillRect(xEnd, area.top, lw, area.bottom - area.top)
    }
  }

  const clipStart = deviceX(transform, sprite.span.start)
  const clipEnd = deviceX(transform, sprite.span.start + sprite.span.length)
  const shade = rgbaToCss(withAlpha(theme.background, 0.42))
  for (const edge of ["in", "out"] as const) {
    const ticks = edge === "in" ? content.fadeIn : content.fadeOut
    if (ticks <= 0) continue
    const width = Math.min(ticks * transform.scaleX, clipEnd - clipStart)
    if (width < 2 * dpr) continue
    const x0 = edge === "in" ? clipStart : clipEnd - width
    const x1 = x0 + width
    // The silent end of the fade, where the two halves meet.
    const tip = edge === "in" ? x0 : x1
    for (const side of [-1, 1] as const) {
      const rim = side === -1 ? area.top : area.bottom
      // What the fade takes away, between the outline and the clip's rim.
      ctx.beginPath()
      fadePath(ctx, edge, x0, x1, area, side, "move")
      ctx.lineTo(edge === "in" ? x1 : x0, rim)
      ctx.lineTo(tip, rim)
      ctx.closePath()
      ctx.fillStyle = shade
      ctx.fill()
      ctx.beginPath()
      fadePath(ctx, edge, x0, x1, area, side, "move")
      ctx.strokeStyle = style.bodyInk
      ctx.lineWidth = lw
      ctx.stroke()
    }
  }

  if (sprite.ghost) return
  const { box } = paint
  const pxPerTick = viewport.pxPerTick
  if (hasFadeHandles(box)) {
    const size = Math.round(FADE_HANDLE * dpr)
    const top = Math.round(fadeHandleTop(box) * dpr)
    for (const edge of ["in", "out"] as const) {
      const ticks = edge === "in" ? content.fadeIn : content.fadeOut
      const left = Math.round(
        fadeHandleLeft(edge, box, ticks * pxPerTick) * dpr
      )
      ctx.fillStyle = style.border
      ctx.fillRect(left, top, size, size)
      ctx.fillStyle = style.bandInk
      ctx.fillRect(left + lw, top + lw, size - 2 * lw, size - 2 * lw)
    }
  }
  if (hasGainHandle(box)) {
    const handle = gainHandleBox(box)
    const left = Math.round(handle.left * dpr)
    const top = Math.round(handle.top * dpr)
    const width = Math.round((handle.right - handle.left) * dpr)
    const height = Math.round((handle.bottom - handle.top) * dpr)
    ctx.fillStyle = style.border
    ctx.fillRect(left, top, width, height)
    ctx.fillStyle = style.bandInk
    ctx.fillRect(left + lw, top + lw, width - 2 * lw, height - 2 * lw)
    // Two notches, so it reads as something to drag up and down.
    ctx.fillStyle = style.band
    const notch = Math.max(lw, Math.round(dpr))
    const middle = left + Math.round(width / 2)
    ctx.fillRect(middle - 2 * notch, top + 2 * lw, notch, height - 4 * lw)
    ctx.fillRect(middle + notch, top + 2 * lw, notch, height - 4 * lw)
  }
}

/** Stripes over a clip whose sample cannot be read. */
export function paintMissing(
  ctx: CanvasRenderingContext2D,
  frame: OverlayFrame,
  sprite: ClipSprite,
  color: string
): void {
  const dpr = frame.viewport.dpr
  const lw = frame.transform.lineWidth
  const left = Math.max(sprite.x0 + lw, 0)
  const right = Math.min(sprite.x1 - lw, frame.transform.widthDev)
  const top = sprite.y0 + lw
  const bottom = sprite.y1 - lw
  if (right <= left || bottom <= top) return
  const height = bottom - top
  const gap = Math.round(7 * dpr)
  ctx.save()
  ctx.beginPath()
  ctx.rect(left, top, right - left, height)
  ctx.clip()
  ctx.globalAlpha = 0.5
  ctx.strokeStyle = color
  ctx.lineWidth = Math.max(lw, Math.round(1.5 * dpr))
  ctx.beginPath()
  for (let x = left - height; x < right; x += gap) {
    ctx.moveTo(x, bottom)
    ctx.lineTo(x + height, top)
  }
  ctx.stroke()
  ctx.restore()
}
