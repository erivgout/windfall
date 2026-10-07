import { Add01Icon } from "@hugeicons/core-free-icons"
import { HugeiconsIcon } from "@hugeicons/react"
import {
  memo,
  useCallback,
  useEffect,
  useRef,
  useState,
  type PointerEvent as ReactPointerEvent,
} from "react"

import type { PlaylistTrack } from "@/bindings"
import { ToggleLed } from "@/components/audio"
import { ActionButton } from "@/components/action-button"
import {
  ContextActions,
  contextSeparator,
  type ContextItem,
} from "@/components/context-actions"
import { useHint } from "@/lib/store/hint"
import { cn } from "@/lib/utils"

import { trackDropIndex } from "./edit"
import { DRAG_THRESHOLD_PX, HEADER_WIDTH } from "./layout"
import { useViewportValue, wheelInput, type GridMetrics } from "./metrics"
import { moveTrack, setTrackName, toggleTrackMute } from "./ops"
import { playlist, usePlaylistTracks } from "./selectors"
import { usePlaylistStore } from "./store"

/** A header's menu acts on the target track; a press on the header sets it. */
const TRACK_MENU: ContextItem[] = [
  "playlist.renameTrack",
  "playlist.muteTrack",
  contextSeparator,
  "playlist.moveTrackUp",
  "playlist.moveTrackDown",
  contextSeparator,
  "playlist.insertTrack",
  "playlist.addTrack",
  contextSeparator,
  "playlist.deleteTrack",
]

const SPARE_MENU: ContextItem[] = ["playlist.addTrack"]

type RowBox = { top: number; height: number }

/**
 * Where the canvas draws a row, in CSS pixels from the top of the scrolled
 * content. The canvas rounds every row line to a device pixel, so the
 * headers are placed by the same rounding instead of by multiplying.
 */
function rowBox(row: number, rowHeight: number, dpr: number): RowBox {
  const top = Math.round(row * rowHeight * dpr) / dpr
  return { top, height: Math.round((row + 1) * rowHeight * dpr) / dpr - top }
}

function NameField({
  track,
  onDone,
}: {
  track: PlaylistTrack
  onDone(): void
}) {
  const [text, setText] = useState(track.name)
  const cancelled = useRef(false)
  return (
    <input
      autoFocus
      aria-label={`Name of ${track.name}`}
      value={text}
      maxLength={64}
      onChange={(event) => setText(event.target.value)}
      onFocus={(event) => event.target.select()}
      onBlur={() => {
        if (!cancelled.current) void setTrackName(track.id, text)
        onDone()
      }}
      onKeyDown={(event) => {
        if (event.key === "Enter") event.currentTarget.blur()
        if (event.key === "Escape") {
          cancelled.current = true
          event.currentTarget.blur()
        }
      }}
      className="h-5 min-w-0 flex-1 rounded-sm border border-ring bg-background px-1 text-xs outline-none"
    />
  )
}

type TrackHeaderProps = {
  track: PlaylistTrack
  row: number
  top: number
  height: number
  target: boolean
  /** The header is being dragged to another place among the tracks. */
  lifted: boolean
  onGrab(event: ReactPointerEvent<HTMLElement>, row: number): void
}

const TrackHeader = memo(function TrackHeader({
  track,
  row,
  top,
  height,
  target,
  lifted,
  onGrab,
}: TrackHeaderProps) {
  const [renaming, setRenaming] = useState(false)
  const setTargetTrack = usePlaylistStore((state) => state.setTargetTrack)
  const hint = useHint(
    `${track.name}${track.muted ? ", muted" : ""}. Drag up or down to reorder, double-click the name to rename, right-click for more`
  )
  const lampHint = useHint(
    `${track.name} is ${track.muted ? "muted" : "on"}. Click to ${track.muted ? "unmute" : "mute"} every clip on it`
  )
  const compact = height < 22

  return (
    <ContextActions items={TRACK_MENU}>
      <div
        role="group"
        aria-label={track.name}
        data-track={track.id}
        data-target={target ? "" : undefined}
        data-lifted={lifted ? "" : undefined}
        onPointerDown={(event) => {
          setTargetTrack(track.id)
          if (!renaming) onGrab(event, row)
        }}
        style={{ top, height }}
        className={cn(
          "absolute inset-x-0 flex items-center gap-1.5 border-t border-(--wf-grid-line) pr-1.5 pl-1 data-lifted:opacity-45",
          target ? "bg-accent/70" : "hover:bg-accent/30"
        )}
        {...hint}
      >
        <span
          aria-hidden
          className="w-5 shrink-0 text-right font-readout text-[0.625rem] text-muted-foreground"
        >
          {row + 1}
        </span>
        <ToggleLed
          variant="dot"
          size={compact ? "sm" : "md"}
          pressed={!track.muted}
          color="var(--wf-meter-low)"
          aria-label={`${track.name} on`}
          onPressedChange={() => void toggleTrackMute(track.id)}
          {...lampHint}
        />
        {renaming ? (
          <NameField track={track} onDone={() => setRenaming(false)} />
        ) : (
          <span
            onDoubleClick={() => setRenaming(true)}
            className={cn(
              "min-w-0 flex-1 truncate",
              track.muted && "text-muted-foreground"
            )}
          >
            {track.name}
          </span>
        )}
      </div>
    </ContextActions>
  )
})

/** A header being dragged: the row it came from and the gap it is over. */
type Reorder = { row: number; gap: number }
type Grab = { row: number; y: number; pointer: number }

const tracksNow = () => playlist().tracks

/** A row with no track yet. Placing a clip on it makes the track. */
const SpareHeader = memo(function SpareHeader({
  row,
  top,
  height,
}: {
  row: number
  top: number
  height: number
}) {
  const hint = useHint(
    `No track here yet. Place a clip on this row and track ${row + 1} is added for it`
  )
  return (
    <ContextActions items={SPARE_MENU}>
      <div
        data-spare-row={row}
        style={{ top, height }}
        className="absolute inset-x-0 flex items-center border-t border-(--wf-grid-line) bg-(--wf-grid-line)/60 pl-1"
        {...hint}
      >
        <span
          aria-hidden
          className="w-5 shrink-0 text-right font-readout text-[0.625rem] text-muted-foreground/60"
        >
          {row + 1}
        </span>
      </div>
    </ContextActions>
  )
})

/** The corner above the headers and left of the ruler. */
export function TrackCorner() {
  return (
    <div className="flex items-center border-r border-b bg-chassis/30 pr-0.5 pl-2 text-muted-foreground">
      <span className="flex-1 text-[0.6875rem] font-medium">Tracks</span>
      <ActionButton
        action="playlist.addTrack"
        variant="ghost"
        size="icon-xs"
        tooltipSide="right"
      >
        <HugeiconsIcon icon={Add01Icon} strokeWidth={2} />
      </ActionButton>
    </div>
  )
}

/**
 * The track names beside the grid. They are ordinary elements, placed where
 * the canvas draws its rows and moved with it: only the rows in view exist,
 * and scrolling shifts their container without rendering anything.
 */
export function TrackHeaders({ metrics }: { metrics: GridMetrics }) {
  const tracks = usePlaylistTracks()
  const target = usePlaylistStore((state) => state.targetTrack)
  const first = useViewportValue(metrics, (viewport) =>
    Math.floor(viewport.scrollRow)
  )
  const shown = useViewportValue(
    metrics,
    (viewport) => Math.ceil(viewport.height / viewport.rowHeight) + 1
  )
  const rowHeight = useViewportValue(metrics, (viewport) => viewport.rowHeight)
  const dpr = useViewportValue(metrics, (viewport) => viewport.dpr)
  const rowCount = useViewportValue(metrics, (_, limits) => limits.rowCount)
  const rootRef = useRef<HTMLDivElement>(null)
  const scrolledRef = useRef<HTMLDivElement>(null)
  const [reorder, setReorder] = useState<Reorder | null>(null)
  const grab = useRef<Grab | null>(null)
  const lifted = useRef<Reorder | null>(null)

  // A press on a header that then moves up or down carries the track to
  // the gap between two others. The lamp and the name keep their clicks.
  const onGrab = useCallback(
    (event: ReactPointerEvent<HTMLElement>, row: number) => {
      const pressed = event.target
      if (
        event.button !== 0 ||
        (pressed instanceof Element && pressed.closest("button, input"))
      ) {
        return
      }
      grab.current = { row, y: event.clientY, pointer: event.pointerId }
    },
    []
  )

  useEffect(() => {
    const root = rootRef.current
    if (!root) return
    const gapAt = (clientY: number) => {
      const { scrollRow, rowHeight } = metrics.viewport
      const y = clientY - root.getBoundingClientRect().top
      return Math.round(scrollRow + y / rowHeight)
    }
    const onMove = (event: PointerEvent) => {
      const held = grab.current
      if (!held || event.pointerId !== held.pointer) return
      const count = tracksNow().length
      if (
        !root.hasPointerCapture(held.pointer) &&
        Math.abs(event.clientY - held.y) < DRAG_THRESHOLD_PX
      ) {
        return
      }
      root.setPointerCapture(held.pointer)
      const gap = Math.min(count, Math.max(0, gapAt(event.clientY)))
      if (lifted.current?.gap === gap) return
      lifted.current = { row: held.row, gap }
      setReorder(lifted.current)
    }
    const finish = (event: PointerEvent, drop: boolean) => {
      const held = grab.current
      if (!held || event.pointerId !== held.pointer) return
      grab.current = null
      if (root.hasPointerCapture(held.pointer)) {
        root.releasePointerCapture(held.pointer)
      }
      const current = lifted.current
      lifted.current = null
      setReorder(null)
      if (!drop || !current) return
      const tracks = tracksNow()
      const index = trackDropIndex(current.row, current.gap, tracks.length)
      const track = tracks[current.row]
      if (index !== null && track) void moveTrack(track.id, index)
    }
    const onUp = (event: PointerEvent) => finish(event, true)
    const onCancel = (event: PointerEvent) => finish(event, false)
    root.addEventListener("pointermove", onMove)
    root.addEventListener("pointerup", onUp)
    root.addEventListener("pointercancel", onCancel)
    return () => {
      root.removeEventListener("pointermove", onMove)
      root.removeEventListener("pointerup", onUp)
      root.removeEventListener("pointercancel", onCancel)
    }
  }, [metrics])

  useEffect(() => {
    const place = () => {
      const scrolled = scrolledRef.current
      if (!scrolled) return
      const { scrollRow, rowHeight, dpr } = metrics.viewport
      const offset = Math.round(scrollRow * rowHeight * dpr) / dpr
      scrolled.style.transform = `translateY(${-offset}px)`
    }
    place()
    return metrics.subscribe(place)
  }, [metrics])

  useEffect(() => {
    const root = rootRef.current
    if (!root) return
    const onWheel = (event: WheelEvent) => {
      event.preventDefault()
      const bounds = root.getBoundingClientRect()
      metrics.wheel(wheelInput(event, { x: 0, y: event.clientY - bounds.top }))
    }
    root.addEventListener("wheel", onWheel, { passive: false })
    return () => root.removeEventListener("wheel", onWheel)
  }, [metrics])

  const rows: React.ReactNode[] = []
  const last = Math.min(rowCount, first + shown)
  for (let row = first; row < last; row++) {
    const track = tracks[row]
    const box = rowBox(row, rowHeight, dpr)
    rows.push(
      track ? (
        <TrackHeader
          key={track.id}
          track={track}
          row={row}
          top={box.top}
          height={box.height}
          target={track.id === target}
          lifted={reorder?.row === row}
          onGrab={onGrab}
        />
      ) : (
        <SpareHeader
          key={`spare-${row}`}
          row={row}
          top={box.top}
          height={box.height}
        />
      )
    )
  }

  return (
    <div
      ref={rootRef}
      role="group"
      aria-label="Tracks"
      className="relative overflow-hidden border-r bg-chassis/20"
      style={{ width: HEADER_WIDTH }}
    >
      <div ref={scrolledRef} className="absolute inset-x-0 top-0">
        {rows}
        {reorder !== null &&
          trackDropIndex(reorder.row, reorder.gap, tracks.length) !== null && (
            <div
              aria-hidden
              data-slot="track-drop-line"
              className="pointer-events-none absolute inset-x-0 z-10 h-0.5 -translate-y-px bg-brand"
              style={{ top: rowBox(reorder.gap, rowHeight, dpr).top }}
            />
          )}
      </div>
    </div>
  )
}
