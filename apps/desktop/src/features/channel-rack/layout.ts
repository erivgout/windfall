import { clamp } from "@/lib/units"

/*
 * Sizes the rows, the ruler and the drop indicator share. Every step has the
 * same width, so the ruler lines up with every row. That width, the pitch,
 * grows to use spare room and stops shrinking at a size that is still easy
 * to hit; a longer pattern then scrolls sideways.
 *
 * The pitch reaches the rows as the CSS variable below, set once on the
 * scrolling area, so resizing the panel renders nothing.
 */

export const STEP_GAP = 2
/** Distance from the left edge of one step to the next, at its smallest. */
export const MIN_STEP_PITCH = 20
export const MAX_STEP_PITCH = 30
export const PITCH_VAR = "--rack-pitch"

export const ROW_HEIGHT = 28
export const RULER_HEIGHT = 22

/** Width of the pinned columns left of the steps. */
export const LEFT_WIDTH = 284
/** Space between the pinned columns and the first step. */
export const STEPS_INSET = 6
/** Space after the last step, so it does not touch the edge. */
export const STEPS_TRAIL = 12

/**
 * The pinned columns: grip, lamp, pan, volume, channel button, mixer track.
 * The header and every row use the same template so captions sit over their
 * column.
 */
export const LEFT_COLUMNS =
  "grid grid-cols-[12px_14px_24px_24px_minmax(0,1fr)_40px] items-center gap-x-1.5 px-1.5"

/** The step pitch that fits `steps` steps into a view `viewWidth` wide. */
export function fitPitch(viewWidth: number, steps: number): number {
  const room = viewWidth - LEFT_WIDTH - STEPS_INSET - STEPS_TRAIL
  return clamp(
    Math.floor(room / Math.max(1, steps)),
    MIN_STEP_PITCH,
    MAX_STEP_PITCH
  )
}

/** A CSS length of `steps` step pitches plus `extra` pixels. */
export function pitches(steps: number, extra = 0): string {
  return `calc(var(${PITCH_VAR}) * ${steps} + ${extra}px)`
}
