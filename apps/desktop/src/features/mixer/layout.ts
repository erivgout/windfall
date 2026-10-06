/** Width of an insert strip in pixels. Every strip is the same width. */
export const STRIP_WIDTH = 80
export const MASTER_WIDTH = 92
/** Width of the "add track" column after the last strip. */
export const ADD_WIDTH = 44

/** Strips kept mounted on each side of the ones in view. */
const OVERSCAN = 2

/*
 * How much of a strip fits at a given panel height, from tall to low.
 *
 * - "full": everything inline, including the output selector and the sends.
 * - "compact": the output and sends move into a popover behind a button in
 *   the header, so the fader keeps a usable length.
 * - "tight": the channel chips join the popover and the peak readout goes.
 * - "flat": too low for an upright fader. It lies across the strip, with
 *   the readouts in a line under it.
 * - "mini": the flat fader with no readouts.
 */
export type StripMode = "full" | "compact" | "tight" | "flat" | "mini"

export type StripLayout = {
  mode: StripMode
  /** Send rows shown inline in "full" mode. Every strip keeps this many. */
  sendRows: number
  /** The fader is too short for every dB mark to have a label. */
  sparseScale: boolean
}

/** Most send rows shown inline. A track with more scrolls its list. */
export const MAX_SEND_ROWS = 3
export const SEND_ROW_HEIGHT = 26

/** The least height an upright fader is useful at. */
const MIN_FADER = 70
/** Below this the labels of the lowest dB marks would overlap. */
const FULL_SCALE_FADER = 144

// What everything but the fader takes up in each mode, in pixels. These
// follow the rows in `strip.tsx`.
const FULL_ROWS = 183
const COMPACT_ROWS = 126
const TIGHT_ROWS = 84
const FLAT_ROWS = 112

/**
 * Picks the layout for strips `height` pixels tall when the busiest track
 * has `maxSends` sends. A height of 0 means "not measured yet" and gets the
 * full layout.
 */
export function stripLayout(height: number, maxSends: number): StripLayout {
  const sendRows = Math.min(Math.max(0, maxSends), MAX_SEND_ROWS)
  if (height <= 0) return { mode: "full", sendRows, sparseScale: false }

  const fullRows = FULL_ROWS + sendRows * SEND_ROW_HEIGHT
  if (height >= fullRows + 110) {
    const sparseScale = height - fullRows < FULL_SCALE_FADER
    return { mode: "full", sendRows, sparseScale }
  }
  if (height >= COMPACT_ROWS + MIN_FADER) {
    const sparseScale = height - COMPACT_ROWS < FULL_SCALE_FADER
    return { mode: "compact", sendRows: 0, sparseScale }
  }
  if (height >= TIGHT_ROWS + MIN_FADER) {
    return { mode: "tight", sendRows: 0, sparseScale: true }
  }
  return {
    mode: height >= FLAT_ROWS ? "flat" : "mini",
    sendRows: 0,
    sparseScale: false,
  }
}

export type StripRange = {
  /** Index of the first strip to mount. */
  start: number
  /** One past the last strip to mount. */
  end: number
}

/**
 * The strips to mount: the ones the scrolled view shows plus a couple on
 * each side, so a strip is ready before it scrolls in.
 */
export function visibleRange(
  scrollLeft: number,
  viewWidth: number,
  count: number
): StripRange {
  const first = Math.floor(Math.max(0, scrollLeft) / STRIP_WIDTH)
  const last = Math.ceil((Math.max(0, scrollLeft) + viewWidth) / STRIP_WIDTH)
  return {
    start: Math.max(0, Math.min(count, first - OVERSCAN)),
    end: Math.max(0, Math.min(count, last + OVERSCAN)),
  }
}

/**
 * Where to scroll so strip `index` is fully in view, or null when it
 * already is.
 */
export function scrollToReveal(
  index: number,
  scrollLeft: number,
  viewWidth: number
): number | null {
  const left = index * STRIP_WIDTH
  const right = left + STRIP_WIDTH
  if (left < scrollLeft) return left
  if (right > scrollLeft + viewWidth) return Math.max(0, right - viewWidth)
  return null
}
