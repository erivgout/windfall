import { BAND_HEIGHT, hasBand, type Box } from "../clip-box"

/*
 * The handles of an audio clip: one for each fade at the top corners of
 * its waveform, under the title bar, and one for the gain at the right end
 * of the title bar, clear of the clip's name. All sizes are CSS pixels.
 */

/** Side of a fade handle. */
export const FADE_HANDLE = 8
/** Width and height of the gain handle. */
export const GAIN_HANDLE_WIDTH = 16
export const GAIN_HANDLE_HEIGHT = 7
/** How far the gain handle sits from the clip's right edge. */
export const GAIN_HANDLE_INSET = 6
/** A clip narrower than this has no fade handles. */
export const MIN_WIDTH_FOR_FADES = 36
/** A clip shorter than this has no room for fade handles over its waveform. */
export const MIN_HEIGHT_FOR_FADES = 32
/** A clip narrower than this has no gain handle. */
export const MIN_WIDTH_FOR_GAIN = 72
/** How far outside a handle a press still takes it. */
const SLOP = 2

export type AudioHandle = "fade-in" | "fade-out" | "gain"

export function hasFadeHandles(box: Box): boolean {
  return (
    hasBand(box) &&
    box.right - box.left >= MIN_WIDTH_FOR_FADES &&
    box.bottom - box.top >= MIN_HEIGHT_FOR_FADES
  )
}

export function hasGainHandle(box: Box): boolean {
  return hasBand(box) && box.right - box.left >= MIN_WIDTH_FOR_GAIN
}

/**
 * Left edge of a fade handle. The handle sits where the fade ends, inside
 * the clip: at the corner while there is no fade, and it never crosses the
 * other half of the clip's handles.
 */
export function fadeHandleLeft(
  edge: "in" | "out",
  box: Box,
  fadePx: number
): number {
  const width = box.right - box.left
  const travel = Math.max(0, width - FADE_HANDLE)
  const along = Math.min(travel, Math.max(0, fadePx - FADE_HANDLE / 2))
  return edge === "in" ? box.left + along : box.right - FADE_HANDLE - along
}

/** Top edge of the fade handles: right under the clip's title bar. */
export function fadeHandleTop(box: Box): number {
  return box.top + 1 + BAND_HEIGHT
}

export function gainHandleBox(box: Box): Box {
  const right = box.right - GAIN_HANDLE_INSET
  const top = box.top + 1 + (BAND_HEIGHT - GAIN_HANDLE_HEIGHT) / 2
  return {
    left: right - GAIN_HANDLE_WIDTH,
    right,
    top,
    bottom: top + GAIN_HANDLE_HEIGHT,
  }
}

function inside(box: Box, x: number, y: number): boolean {
  return (
    x >= box.left - SLOP &&
    x < box.right + SLOP &&
    y >= box.top - SLOP &&
    y < box.bottom + SLOP
  )
}

/**
 * The handle of an audio clip under a point, or null. `fadeInPx` and
 * `fadeOutPx` are the clip's fades as widths on screen. Where the two fade
 * handles overlap, the nearer one is taken.
 */
export function hitAudioHandle(
  box: Box,
  fadeInPx: number,
  fadeOutPx: number,
  x: number,
  y: number
): AudioHandle | null {
  const top = fadeHandleTop(box)
  if (hasFadeHandles(box) && y >= top - SLOP && y < top + FADE_HANDLE + SLOP) {
    const inLeft = fadeHandleLeft("in", box, fadeInPx)
    const outLeft = fadeHandleLeft("out", box, fadeOutPx)
    const onIn = x >= inLeft - SLOP && x < inLeft + FADE_HANDLE + SLOP
    const onOut = x >= outLeft - SLOP && x < outLeft + FADE_HANDLE + SLOP
    if (onIn && onOut) {
      const center = x - FADE_HANDLE / 2
      return Math.abs(center - inLeft) <= Math.abs(center - outLeft)
        ? "fade-in"
        : "fade-out"
    }
    if (onIn) return "fade-in"
    if (onOut) return "fade-out"
  }
  if (hasGainHandle(box) && inside(gainHandleBox(box), x, y)) return "gain"
  return null
}
