import type { Clip, ClipId } from "@/bindings"
import type { Viewport } from "@/lib/canvas"
import { useUiStore } from "@/lib/store/ui"

import { usePlaylistStore } from "./store"

/**
 * Brings the playlist forward with a clip selected and in view. The panel
 * may not be on screen yet, so the clip is remembered and the timeline
 * scrolls to it once it is.
 *
 * With `focus` the timeline takes the keyboard too, so the next key acts
 * on the clip that is now selected and not on the control the command was
 * given from: Delete after "Create automation clip" deletes the clip.
 */
export function revealClip(
  id: ClipId,
  options: { focus?: boolean } = {}
): void {
  revealClips([id], options)
}

/** The same for several clips: all selected, the first one in view. */
export function revealClips(
  ids: readonly ClipId[],
  options: { focus?: boolean } = {}
): void {
  if (ids.length === 0) return
  const playlist = usePlaylistStore.getState()
  playlist.select(ids)
  playlist.setReveal(ids[0])
  if (options.focus) playlist.requestFocus()
  useUiStore.getState().showCenterTab("playlist")
}

/** Bars of air left before a clip that is scrolled into view. */
const LEAD_TICKS = 3840 / 2

/**
 * The viewport that shows a clip on `row`: unchanged when the clip's start
 * and its row are already in view, otherwise scrolled so the clip starts
 * near the left edge and its row is on screen.
 */
export function viewportShowing(
  viewport: Viewport,
  clip: Pick<Clip, "start" | "length">,
  row: number
): Viewport {
  const visibleTicks = viewport.width / viewport.pxPerTick
  const visibleRows = viewport.height / viewport.rowHeight
  const startShown =
    clip.start >= viewport.scrollTick &&
    clip.start < viewport.scrollTick + visibleTicks * 0.9
  const rowShown =
    row >= viewport.scrollRow && row + 1 <= viewport.scrollRow + visibleRows
  return {
    ...viewport,
    scrollTick: startShown
      ? viewport.scrollTick
      : Math.max(0, clip.start - LEAD_TICKS),
    scrollRow: rowShown
      ? viewport.scrollRow
      : Math.max(0, row - Math.max(0, Math.floor(visibleRows / 2) - 1)),
  }
}
