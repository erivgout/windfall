import type { Clip, ClipId, Pattern, PatternId } from "@/bindings"
import {
  mix,
  RECT_SELECTED,
  RectBatch,
  rgbFromInt,
  type GridTheme,
  type Rgba,
} from "@/lib/canvas"

import type { RowOf } from "./edit"

/** Shown for a clip whose pattern cannot be found. It should never be seen. */
export const ORPHAN_COLOR = 0x8b8d98

/**
 * The fill of a clip's body. It is the pattern's color pulled toward the
 * background, which leaves room for a title bar in the full color on top
 * and reads as "this color" in both themes. A muted clip fades almost out.
 */
export function clipBodyColor(
  theme: GridTheme,
  color: number,
  muted: boolean
): Rgba {
  return mix(theme.background, rgbFromInt(color), muted ? 0.24 : 0.62)
}

/**
 * Builds the rects the grid draws for the clips: one per clip, on its
 * track's row. Clips arrive sorted by start, which the grid's index needs.
 * Build again when the clips, the patterns' colors, the track mutes or the
 * theme change. A change of selection only needs the flags set.
 */
export function buildClipBatch(
  clips: readonly Clip[],
  rowOf: RowOf,
  patterns: ReadonlyMap<PatternId, Pattern>,
  isMuted: (clip: Clip) => boolean,
  selection: ReadonlySet<ClipId>,
  theme: GridTheme
): RectBatch {
  const batch = new RectBatch(Math.max(16, clips.length))
  const colors = new Map<number, Rgba>()
  for (const clip of clips) {
    const muted = isMuted(clip)
    const color = patterns.get(clip.content.pattern)?.color ?? ORPHAN_COLOR
    const key = color * 2 + (muted ? 1 : 0)
    let fill = colors.get(key)
    if (!fill) {
      fill = clipBodyColor(theme, color, muted)
      colors.set(key, fill)
    }
    batch.push(
      clip.id,
      clip.start,
      clip.length,
      rowOf(clip.track),
      1,
      fill,
      selection.has(clip.id) ? RECT_SELECTED : 0
    )
  }
  return batch
}
