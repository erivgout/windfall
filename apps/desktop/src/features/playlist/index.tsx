import { useEffect, useMemo, useRef } from "react"

import { useTransportStore } from "@/lib/store/transport"

import { setActiveMetrics } from "./active"
import { PlaylistGrid } from "./grid"
import { installPlaylistKeys, installSpacePlays } from "./keys"
import { HEADER_WIDTH, RULER_HEIGHT, SCROLLBAR_SIZE } from "./layout"
import { GridMetrics } from "./metrics"
import { applySongCursor } from "./ops"
import { PatternPicker } from "./pattern-picker"
import { Ruler } from "./ruler"
import { Scrollbar } from "./scrollbar"
import { usePlaylistStore } from "./store"
import { PlaylistToolbar } from "./toolbar"
import { TrackCorner, TrackHeaders } from "./track-headers"

const GRID_TEMPLATE = {
  gridTemplateColumns: `${HEADER_WIDTH}px minmax(0, 1fr) ${SCROLLBAR_SIZE}px`,
  gridTemplateRows: `${RULER_HEIGHT}px minmax(0, 1fr) ${SCROLLBAR_SIZE}px`,
}

/**
 * The playlist: the song's timeline. Patterns made in the channel rack and
 * the piano roll are laid out here as clips on tracks. A toolbar on top,
 * the patterns to place at the left, then track names, a bar ruler and the
 * clip grid.
 */
export default function PlaylistPanel() {
  const rootRef = useRef<HTMLDivElement>(null)
  const metrics = useMemo(() => new GridMetrics(), [])
  const pickerOpen = usePlaylistStore((state) => state.pickerOpen)

  useEffect(() => {
    const root = rootRef.current
    if (!root) return
    const stops = [
      setActiveMetrics(metrics),
      installPlaylistKeys(root),
      installSpacePlays(root),
      // However song mode is switched on, the song starts where the ruler
      // was last set.
      useTransportStore.subscribe((state, previous) => {
        if (state.mode === "song" && previous.mode !== "song") {
          void applySongCursor()
        }
      }),
    ]
    return () => {
      for (const stop of stops) stop()
    }
  }, [metrics])

  return (
    <div
      ref={rootRef}
      data-slot="playlist"
      className="flex h-full min-h-0 min-w-0 flex-col"
    >
      <PlaylistToolbar />
      <div className="flex min-h-0 flex-1">
        {pickerOpen && <PatternPicker />}
        <div className="grid min-h-0 min-w-0 flex-1" style={GRID_TEMPLATE}>
          <TrackCorner />
          <Ruler metrics={metrics} />
          <div className="border-b border-l bg-chassis/30" />
          <TrackHeaders metrics={metrics} />
          <PlaylistGrid metrics={metrics} />
          <Scrollbar metrics={metrics} axis="tracks" />
          <div className="border-t border-r bg-chassis/30" />
          <Scrollbar metrics={metrics} axis="time" />
          <div className="border-t border-l bg-chassis/30" />
        </div>
      </div>
    </div>
  )
}
