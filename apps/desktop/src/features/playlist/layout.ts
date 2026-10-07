/** Sizes the playlist's parts share, in CSS pixels unless a name says ticks. */

export const HEADER_WIDTH = 136
export const PICKER_WIDTH = 152
export const RULER_HEIGHT = 24
export const SCROLLBAR_SIZE = 10

export const DEFAULT_ROW_HEIGHT = 38
export const MIN_ROW_HEIGHT = 18
export const MAX_ROW_HEIGHT = 96

/** A bar is this wide when the playlist first opens. */
export const DEFAULT_BAR_WIDTH = 72
export const MIN_PX_PER_TICK = 0.0006
export const MAX_PX_PER_TICK = 0.4

/** Rows shown even in a project with no tracks, so a clip can go anywhere. */
export const MIN_ROWS = 12
/** Empty rows kept below the last track for new tracks to start in. */
export const SPARE_ROWS = 8

/** Empty bars kept to the right of the last clip. */
export const TAIL_BARS = 16
export const MIN_SONG_BARS = 64

/** How close to a clip's end the pointer resizes instead of moving it. */
export const EDGE_PX = 6
/** A press that moves less than this is a click, not a drag. */
export const DRAG_THRESHOLD_PX = 3
export const DOUBLE_CLICK_MS = 400

/**
 * How many rows the grid has: every track, some spare ones below, and never
 * fewer than fill the view.
 */
export function rowCountFor(trackCount: number, visibleRows: number): number {
  return Math.max(MIN_ROWS, trackCount + SPARE_ROWS, Math.ceil(visibleRows))
}

/** Length of the scrollable timeline: the song plus empty bars after it. */
export function contentTicksFor(songEnd: number, barTicks: number): number {
  const songBars = Math.ceil(songEnd / barTicks)
  return Math.max(MIN_SONG_BARS, songBars + TAIL_BARS) * barTicks
}

/** A drag held this close to an edge of the grid scrolls it. */
export const EDGE_SCROLL_MARGIN = 20
/** The most an edge scrolls per frame. */
export const EDGE_SCROLL_MAX = 28

/**
 * Pixels to scroll this frame for a drag at `position` along a side `size`
 * long: nothing in the middle, faster the further past the margin it is.
 */
export function edgePull(position: number, size: number): number {
  const speed = (over: number) =>
    Math.min(EDGE_SCROLL_MAX, Math.ceil(over * 0.4))
  if (position < EDGE_SCROLL_MARGIN) {
    return -speed(EDGE_SCROLL_MARGIN - position)
  }
  if (position > size - EDGE_SCROLL_MARGIN) {
    return speed(position - (size - EDGE_SCROLL_MARGIN))
  }
  return 0
}
