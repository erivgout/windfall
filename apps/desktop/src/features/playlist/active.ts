import { songEnd } from "./edit"
import type { GridMetrics } from "./metrics"
import { playlist, project } from "./selectors"
import type { PlaylistSession } from "./session"
import { ticksPerBar } from "@/lib/time"

/*
 * The playlist panel that is on screen, for the actions that need it: the
 * zoom actions change its viewport, and Escape drops its drag. There is at
 * most one, because the panel is one tab of the center dock.
 */

let metrics: GridMetrics | null = null
let session: PlaylistSession | null = null

export function activeMetrics(): GridMetrics | null {
  return metrics
}

export function activeSession(): PlaylistSession | null {
  return session
}

/** Called by the panel while it is mounted. Returns a function that undoes it. */
export function setActiveMetrics(next: GridMetrics): () => void {
  metrics = next
  return () => {
    if (metrics === next) metrics = null
  }
}

export function setActiveSession(next: PlaylistSession): () => void {
  session = next
  return () => {
    if (session === next) session = null
  }
}

/** Shows the whole song, with a bar of air after it, from the top track. */
export function zoomToFit(): void {
  if (!metrics) return
  const bar = ticksPerBar(project().settings.timeSignature)
  const viewport = metrics.viewport
  const shown = Math.max(songEnd(playlist().clips), 4 * bar) + bar
  metrics.setViewport({
    ...viewport,
    pxPerTick: viewport.width / shown,
    scrollTick: 0,
    scrollRow: 0,
  })
}

/** Zooms time around the middle of the view. */
export function zoomBy(factor: number): void {
  if (!metrics) return
  metrics.zoomTime(metrics.viewport.width / 2, factor)
}
