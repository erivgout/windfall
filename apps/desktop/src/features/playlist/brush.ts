import type { TrackId } from "@/bindings"
import { MASTER_TRACK } from "@/lib/units"

import { wholeClipTicks } from "./audio/geometry"
import { patternTicks, type NewClip } from "./edit"
import type { ResolvedBrush } from "./selectors"

/** What the length of a brush's clip depends on besides the brush itself. */
export type BrushContext = {
  /** One bar of the song, in ticks. */
  barTicks: number
  tempoBpm: number
  /** Length of an audio brush's sample, or null while it is not known. */
  durationSecs: number | null
  /** The mixer track an audio brush's clips play into. */
  mixerTrack?: TrackId
}

/**
 * How long one clip of the brush is: one pass of a pattern, a sample for
 * as long as its audio lasts at the tempo now, an automation up to its
 * last point and at least a bar. A sample that has not been read yet
 * stands in at a bar; the clip gets its real length when it is placed.
 */
export function brushTicks(
  brush: ResolvedBrush,
  context: BrushContext
): number {
  switch (brush.type) {
    case "pattern":
      return patternTicks(brush.pattern)
    case "audio":
      return context.durationSecs === null
        ? context.barTicks
        : wholeClipTicks(context.durationSecs, context.tempoBpm)
    case "automation":
      return Math.max(
        context.barTicks,
        brush.automation.points.at(-1)?.tick ?? 0
      )
    default: {
      const _exhaustive: never = brush
      return _exhaustive
    }
  }
}

/** A clip of the brush at a place on the timeline. */
export function brushClip(
  brush: ResolvedBrush,
  start: number,
  row: number,
  context: BrushContext
): NewClip {
  const base = {
    row,
    start,
    length: brushTicks(brush, context),
    offset: 0,
    muted: false,
  }
  switch (brush.type) {
    case "pattern":
      return {
        ...base,
        content: { type: "pattern", pattern: brush.pattern.id },
      }
    case "audio":
      return {
        ...base,
        content: {
          type: "audio",
          sample: brush.sample.id,
          mixerTrack: context.mixerTrack ?? MASTER_TRACK,
          gain: 1,
          pan: 0,
          fadeIn: 0,
          fadeOut: 0,
          reverse: false,
          pitch: 0,
        },
      }
    case "automation":
      return {
        ...base,
        content: { type: "automation", automation: brush.automation.id },
      }
    default: {
      const _exhaustive: never = brush
      return _exhaustive
    }
  }
}

/** The brush in words, for hints: "Kick loop", "the Cutoff curve". */
export function brushName(brush: ResolvedBrush | null): string {
  if (!brush) return "a pattern"
  switch (brush.type) {
    case "pattern":
      return brush.pattern.name
    case "audio":
      return brush.sample.name
    case "automation":
      return brush.automation.name
    default: {
      const _exhaustive: never = brush
      return _exhaustive
    }
  }
}
