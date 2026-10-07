import { useCallback, useEffect, useMemo, useRef, useState } from "react"

import {
  ContextActions,
  contextSeparator,
  type ContextItem,
} from "@/components/context-actions"
import { isEnabled, getAppState, registry, runAction } from "@/lib/actions"
import {
  plainRows,
  type TimeGridView,
  type TimeGridViewOptions,
} from "@/lib/canvas"
import { TimeGridCanvas } from "@/lib/canvas/TimeGridCanvas"
import { errorMessage } from "@/lib/ipc"
import { usePlayhead } from "@/lib/store/realtime"
import { usePattern, useSelectedPatternId } from "@/lib/store/selectors"
import { useTransportStore } from "@/lib/store/transport"

import { setActiveSession } from "./active"
import type { Tool } from "./intents"
import { playlistShortcut } from "./keys"
import { MIN_ROWS } from "./layout"
import { initialView, PLAYLIST_LIMITS, type GridMetrics } from "./metrics"
import { attachPointer } from "./pointer"
import { useClipCount, useSelectionCount } from "./selectors"
import { PlaylistSession } from "./session"
import { gridSpecFor } from "./snap"
import { usePlaylistStore } from "./store"
import { useLiveHint } from "./use-live-hint"

/** An action of the playlist as a menu entry, with the key it has in here. */
function menuEntry(id: string): ContextItem {
  const action = registry.get(id)
  if (!action) return id
  return {
    title: action.title,
    run: () => runAction(id),
    disabled: !isEnabled(action, getAppState()),
    shortcut: playlistShortcut(id),
    destructive: id === "playlist.deleteClips",
  }
}

const CLIP_MENU = [
  "playlist.editPattern",
  null,
  "playlist.cut",
  "playlist.copy",
  "playlist.duplicate",
  "playlist.muteClips",
  null,
  "playlist.deleteClips",
]

const EMPTY_MENU = [
  "playlist.paste",
  "playlist.selectAll",
  null,
  "playlist.zoomToFit",
  "playlist.addTrack",
]

function hintFor(tool: Tool, pattern: string): string {
  const edit =
    "Drag a clip to move it, its edges to resize. Shift+drag copies. Ctrl+drag selects"
  switch (tool) {
    case "draw":
      return `Click to place ${pattern}. ${edit}. Right-click deletes`
    case "paint":
      return `Drag to paint ${pattern} back to back. ${edit}. Right-click deletes`
    case "select":
      return "Drag a box to select clips, Shift adds to it. Drag a clip to move it, its edges to resize. Shift+drag copies. Right-click for a menu"
    case "erase":
      return "Click a clip to delete it, or drag across several"
    case "mute":
      return "Click a clip to mute or unmute it, or drag across several. Right-click deletes"
    default: {
      const _exhaustive: never = tool
      return _exhaustive
    }
  }
}

/** How far from the left edge the playhead lands when the view follows it. */
const FOLLOW_LEAD = 0.1

/**
 * The clip grid: the canvas, the session that keeps it current and turns
 * the pointer into edits, the playhead, and the right-click menu of the
 * Select tool.
 */
export function PlaylistGrid({ metrics }: { metrics: GridMetrics }) {
  const [failure, setFailure] = useState<string | null>(null)
  const containerRef = useRef<HTMLDivElement>(null)
  const viewRef = useRef<TimeGridView | null>(null)
  const cleanupRef = useRef<(() => void) | null>(null)
  const tool = usePlaylistStore((state) => state.tool)
  const menuOnClips = usePlaylistStore((state) => state.menuOnClips)
  const clipCount = useClipCount()
  const brush = usePattern(useSelectedPatternId())
  // Read so the menu's entries follow what is selected and copied.
  useSelectionCount()
  usePlaylistStore((state) => state.clipboard)
  const hint = useLiveHint(hintFor(tool, brush?.name ?? "a pattern"))

  // The view reads its options once. The session sets everything that can
  // change afterwards, so these are only the values to start with.
  const options = useMemo<TimeGridViewOptions>(
    () => ({
      renderer: "auto",
      limits: PLAYLIST_LIMITS,
      rows: plainRows(MIN_ROWS),
      timeGrid: gridSpecFor(usePlaylistStore.getState().snap, {
        numerator: 4,
        denominator: 4,
      }),
      initial: initialView(),
    }),
    []
  )

  const onReady = useCallback(
    (view: TimeGridView) => {
      const detach = metrics.attach(view)
      const session = new PlaylistSession(view, metrics)
      session.onCursor = (cursor) => {
        view.element.style.cursor = cursor
      }
      const stops = [
        attachPointer(view.element, session, metrics, {
          localPoint: (event) => view.localPoint(event),
          focus: () => containerRef.current?.focus({ preventScroll: true }),
        }),
        setActiveSession(session),
        () => session.destroy(),
        detach,
      ]
      viewRef.current = view
      // Says which renderer drew the grid, for bug reports and measurements.
      containerRef.current?.setAttribute(
        "data-renderer",
        view.renderer.info.kind
      )
      cleanupRef.current = () => {
        for (const stop of stops) stop()
        viewRef.current = null
      }
    },
    [metrics]
  )

  const onError = useCallback((error: unknown) => {
    setFailure(errorMessage(error))
  }, [])

  useEffect(
    () => () => {
      cleanupRef.current?.()
      cleanupRef.current = null
    },
    []
  )

  usePlayhead((tick, playing) => {
    const view = viewRef.current
    const transport = useTransportStore.getState()
    const ui = usePlaylistStore.getState()
    if (transport.mode !== "song") {
      // The playhead is inside the pattern now, not on the playlist.
      view?.setPlayhead(null)
      return
    }
    view?.setPlayhead(tick)
    // While stopped the playhead rests where the song will start from.
    if (!playing) ui.setCursorTick(tick)
    if (!playing || !ui.follow) return
    const viewport = metrics.viewport
    const visible = viewport.width / viewport.pxPerTick
    if (tick < viewport.scrollTick || tick > viewport.scrollTick + visible) {
      metrics.setViewport({
        ...viewport,
        scrollTick: tick - visible * FOLLOW_LEAD,
      })
    }
  })

  const items: ContextItem[] = (menuOnClips ? CLIP_MENU : EMPTY_MENU).map(
    (id) => (id === null ? contextSeparator : menuEntry(id))
  )

  return (
    <ContextActions items={items}>
      <div
        ref={containerRef}
        tabIndex={0}
        role="application"
        aria-label="Song timeline"
        aria-roledescription="clip grid"
        data-slot="playlist-grid"
        data-tool={tool}
        className="relative min-h-0 min-w-0 touch-none overflow-hidden focus-visible:-outline-offset-2"
        {...hint}
      >
        <TimeGridCanvas
          className="absolute inset-0"
          options={options}
          onReady={onReady}
          onError={onError}
        />
        {failure !== null && (
          <p
            role="alert"
            className="absolute inset-x-0 top-1/3 mx-auto max-w-sm text-center text-muted-foreground"
          >
            The timeline could not be drawn. {failure}
          </p>
        )}
        {clipCount === 0 && failure === null && (
          <div className="pointer-events-none absolute inset-0 flex items-center justify-center p-6">
            <div className="max-w-xs rounded-lg border bg-popover/85 px-4 py-3 text-center shadow-xs">
              <p className="font-medium text-foreground">The song is empty</p>
              <p className="mt-1 text-muted-foreground">
                Pick a pattern on the left and click in the timeline to place
                it. Every row can hold clips; tracks are added as you go.
              </p>
            </div>
          </div>
        )}
      </div>
    </ContextActions>
  )
}
