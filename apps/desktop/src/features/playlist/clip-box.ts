import { rowToY, tickToX, type Viewport } from "@/lib/canvas"

/*
 * Where a clip and its parts are on screen, in CSS pixels from the grid's
 * top left corner. The painter draws by these and the pointer is tested
 * against them, so a handle is where it looks to be.
 */

/** Height of the title bar on top of a clip. */
export const BAND_HEIGHT = 13
/** A clip shorter than this has no title bar; its name goes in its body. */
export const MIN_HEIGHT_FOR_BAND = 25
/** Room left above and below what is drawn inside a clip. */
export const CONTENT_PAD = 3

export type Box = {
  readonly left: number
  readonly right: number
  readonly top: number
  readonly bottom: number
}

/** The whole rect of a clip on a row. */
export function clipBox(
  viewport: Viewport,
  span: { start: number; length: number },
  row: number
): Box {
  const top = rowToY(viewport, row)
  return {
    left: tickToX(viewport, span.start),
    right: tickToX(viewport, span.start + span.length),
    top: top + 1,
    bottom: top + viewport.rowHeight,
  }
}

export function hasBand(box: Box): boolean {
  return box.bottom - box.top >= MIN_HEIGHT_FOR_BAND && box.right - box.left > 2
}

/** The part of a clip under its title bar, where its content is drawn. */
export function contentArea(box: Box): Box {
  return {
    left: box.left + 1,
    right: box.right - 1,
    top: box.top + 1 + (hasBand(box) ? BAND_HEIGHT : 0) + CONTENT_PAD,
    bottom: box.bottom - 1 - CONTENT_PAD,
  }
}

/** A box in device pixels, for drawing. */
export function scaleBox(box: Box, dpr: number): Box {
  return {
    left: Math.round(box.left * dpr),
    right: Math.round(box.right * dpr),
    top: Math.round(box.top * dpr),
    bottom: Math.round(box.bottom * dpr),
  }
}
