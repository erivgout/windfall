import { logicalDelta } from "@/lib/ui-scale"
import { useEffect, useRef } from "react"

import type { TimeSignature } from "@/bindings"
import {
  observeCanvas,
  resolveColors,
  subscribeTheme,
  type CanvasSize,
} from "@/components/audio"
import {
  ContextActions,
  contextSeparator,
  type ContextItem,
} from "@/components/context-actions"
import {
  deviceTransform,
  deviceX,
  tickToX,
  visibleTicks,
  xToTick,
  type Viewport,
} from "@/lib/canvas"
import { ignoresSnap, MODIFIER_HINTS } from "@/lib/edit-modifiers"
import { useHint } from "@/lib/store/hint"
import { useProjectStore } from "@/lib/store/project"
import { usePlayhead } from "@/lib/store/realtime"
import { useSettings } from "@/lib/store/selectors"
import { useTransportStore } from "@/lib/store/transport"
import { ticksPerBar, ticksPerBeat } from "@/lib/time"

import { snapNearest, songEnd } from "./edit"
import { wheelInput, type GridMetrics } from "./metrics"
import { currentSnapTicks, seekSong, songTick, toggleLoopSong } from "./ops"
import { usePlaylistStore } from "./store"

const RULER_COLORS = {
  ink: "var(--muted-foreground)",
  line: "var(--wf-grid-line-strong)",
  after: "color-mix(in oklch, var(--foreground) 7%, transparent)",
  end: "var(--muted-foreground)",
}

type RulerColors = typeof RULER_COLORS

/** Bar numbers get at least this much room each, in CSS pixels. */
const MIN_LABEL_SPACING = 46
const MIN_BEAT_SPACING = 7
const MIN_BAR_SPACING = 5

/** How many bars one number stands for: 1 zoomed in, then 2, 4, 8 and so on. */
export function barsPerLabel(barPx: number): number {
  let bars = 1
  while (bars * barPx < MIN_LABEL_SPACING && bars < 4096) bars *= 2
  return bars
}

/**
 * Draws the bar numbers, the beat marks and the end of the song. Lines are
 * placed with the grid's own rounding, so they sit exactly above its lines.
 */
function drawRuler(
  ctx: CanvasRenderingContext2D,
  size: CanvasSize,
  viewport: Viewport,
  signature: TimeSignature,
  end: number,
  colors: RulerColors
): void {
  const { pixelWidth: width, pixelHeight: height, dpr } = size
  const transform = deviceTransform({ ...viewport, dpr })
  const lw = transform.lineWidth
  const bar = ticksPerBar(signature)
  const beat = ticksPerBeat(signature)
  const ticks = visibleTicks(viewport)
  ctx.setTransform(1, 0, 0, 1, 0, 0)
  ctx.clearRect(0, 0, width, height)

  const endX = deviceX(transform, end)
  if (end > 0 && endX < width) {
    ctx.fillStyle = colors.after
    ctx.fillRect(Math.max(0, endX), 0, width - Math.max(0, endX), height)
  }

  ctx.fillStyle = colors.line
  if (beat * viewport.pxPerTick >= MIN_BEAT_SPACING) {
    const mark = Math.round(4 * dpr)
    const first = Math.max(0, Math.floor(ticks.start / beat) * beat)
    for (let tick = first; tick <= ticks.end; tick += beat) {
      if (tick % bar === 0) continue
      ctx.fillRect(deviceX(transform, tick), height - mark, lw, mark)
    }
  }

  const barPx = bar * viewport.pxPerTick
  const labelEvery = barsPerLabel(barPx)
  const step = barPx >= MIN_BAR_SPACING ? 1 : labelEvery
  const firstBar = Math.floor(Math.max(0, ticks.start) / bar / step) * step
  const shortMark = Math.round(8 * dpr)
  ctx.font = `500 ${Math.round(10 * dpr)}px "Inter Variable", system-ui, sans-serif`
  ctx.textBaseline = "middle"
  for (let index = firstBar; index * bar <= ticks.end; index += step) {
    const x = deviceX(transform, index * bar)
    const labelled = index % labelEvery === 0
    ctx.fillStyle = colors.line
    if (labelled) ctx.fillRect(x, 0, lw, height)
    else ctx.fillRect(x, height - shortMark, lw, shortMark)
    if (labelled) {
      ctx.fillStyle = colors.ink
      ctx.fillText(
        String(index + 1),
        x + Math.round(4 * dpr),
        Math.round(height / 2 - dpr)
      )
    }
  }

  if (end > 0 && endX >= 0 && endX < width) {
    // A small flag where the last clip ends.
    const flag = Math.round(6 * dpr)
    ctx.fillStyle = colors.end
    ctx.fillRect(endX, 0, 2 * lw, height)
    ctx.beginPath()
    ctx.moveTo(endX + 2 * lw, 0)
    ctx.lineTo(endX + 2 * lw + flag, flag / 2)
    ctx.lineTo(endX + 2 * lw, flag)
    ctx.fill()
  }
}

/** Moves the marker to a tick, or hides it when that is out of view. */
function placeMarker(
  marker: HTMLElement | null,
  shown: { current: number | null },
  metrics: GridMetrics,
  tick: number
): void {
  if (!marker) return
  const viewport = metrics.viewport
  const raw = tickToX(viewport, tick)
  const x =
    raw < -8 || raw > viewport.width + 8
      ? null
      : Math.round(raw * viewport.dpr) / viewport.dpr
  if (x === shown.current) return
  shown.current = x
  marker.style.visibility = x === null ? "hidden" : "visible"
  if (x !== null) marker.style.transform = `translateX(${x}px)`
}

/**
 * The bar ruler above the grid. Click or drag in it to move the song
 * position; it also shows where the song ends.
 *
 * The marker is the song position. In song mode it is the playhead and
 * moves while the song plays. In pattern mode the transport's playhead
 * runs inside the pattern instead, so the marker is dimmed and stays where
 * the song will start from.
 */
export function Ruler({ metrics }: { metrics: GridMetrics }) {
  const signature = useSettings().timeSignature
  const end = useProjectStore((state) => songEnd(state.project.playlist.clips))
  const mode = useTransportStore((state) => state.mode)
  const loop = useTransportStore((state) => state.loopSong)
  const cursor = usePlaylistStore((state) => state.cursorTick)
  const follow = usePlaylistStore((state) => state.follow)
  const toggleFollow = usePlaylistStore((state) => state.toggleFollow)
  const rootRef = useRef<HTMLDivElement>(null)
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const markerRef = useRef<HTMLDivElement>(null)
  const markerX = useRef<number | null>(null)
  const lastSeek = useRef<number | null>(null)
  const hint = useHint(
    mode === "song"
      ? `Click or drag to move the playhead. ${MODIFIER_HINTS.free}`
      : "Click to set where the song starts. The transport is looping the pattern, so the song is not playing"
  )

  usePlayhead((tick) => {
    if (useTransportStore.getState().mode !== "song") return
    placeMarker(markerRef.current, markerX, metrics, tick)
  })

  // The marker also has to move when the mode or the cursor changes.
  useEffect(() => {
    const place = () =>
      placeMarker(markerRef.current, markerX, metrics, songTick())
    place()
    return metrics.subscribe(place)
  }, [metrics, mode, cursor])

  useEffect(() => {
    const canvas = canvasRef.current
    const ctx = canvas?.getContext("2d")
    if (!canvas || !ctx) return
    let colors = resolveColors(canvas, RULER_COLORS)
    let size: CanvasSize | null = null
    let frame = 0
    const draw = () => {
      frame = 0
      if (size) drawRuler(ctx, size, metrics.viewport, signature, end, colors)
    }
    // Several scroll events can arrive within one frame.
    const schedule = () => {
      if (frame === 0) frame = requestAnimationFrame(draw)
    }
    const stops = [
      observeCanvas(canvas, (next) => {
        size = next
        draw()
      }),
      metrics.subscribe(schedule),
      subscribeTheme(() => {
        colors = resolveColors(canvas, RULER_COLORS)
        schedule()
      }),
    ]
    return () => {
      cancelAnimationFrame(frame)
      for (const stop of stops) stop()
    }
  }, [metrics, signature, end])

  useEffect(() => {
    const root = rootRef.current
    if (!root) return
    const onWheel = (event: WheelEvent) => {
      event.preventDefault()
      const bounds = root.getBoundingClientRect()
      const input = wheelInput(event, {
        x: logicalDelta(event.clientX - bounds.left),
        y: 0,
      })
      // Over the ruler a plain wheel scrolls time, which is what it shows.
      metrics.wheel({ ...input, shift: !input.mod })
    }
    root.addEventListener("wheel", onWheel, { passive: false })
    return () => root.removeEventListener("wheel", onWheel)
  }, [metrics])

  const seekAt = (event: React.PointerEvent<HTMLDivElement>) => {
    const bounds = event.currentTarget.getBoundingClientRect()
    const tick = xToTick(
      metrics.viewport,
      logicalDelta(event.clientX - bounds.left)
    )
    const target = snapNearest(
      tick,
      ignoresSnap({ alt: event.altKey }) ? 0 : currentSnapTicks()
    )
    if (target === lastSeek.current) return
    lastSeek.current = target
    void seekSong(target)
  }

  const items: ContextItem[] = [
    { title: "Jump to the start", run: () => seekSong(0) },
    {
      title: "Jump to the end of the song",
      run: () => seekSong(end),
      disabled: end === 0,
    },
    contextSeparator,
    "playlist.zoomToFit",
    {
      title: loop ? "Stop at the end of the song" : "Loop the song",
      run: toggleLoopSong,
    },
    {
      title: follow ? "Stop following the playhead" : "Follow the playhead",
      run: toggleFollow,
    },
  ]

  return (
    <ContextActions items={items}>
      <div
        ref={rootRef}
        role="slider"
        aria-label="Song position"
        aria-valuemin={0}
        aria-valuemax={Math.max(end, 1)}
        aria-valuenow={cursor}
        aria-valuetext={`Bar ${Math.floor(cursor / ticksPerBar(signature)) + 1}`}
        data-mode={mode}
        className="relative cursor-pointer overflow-hidden border-b bg-chassis/30"
        onPointerDown={(event) => {
          if (event.button !== 0) return
          event.currentTarget.setPointerCapture(event.pointerId)
          lastSeek.current = null
          seekAt(event)
        }}
        onPointerMove={(event) => {
          if (event.currentTarget.hasPointerCapture(event.pointerId)) {
            seekAt(event)
          }
        }}
        onPointerUp={(event) => {
          if (event.currentTarget.hasPointerCapture(event.pointerId)) {
            event.currentTarget.releasePointerCapture(event.pointerId)
          }
        }}
        {...hint}
      >
        <canvas ref={canvasRef} className="absolute inset-0 size-full" />
        <div
          ref={markerRef}
          aria-hidden
          data-slot="song-marker"
          className="pointer-events-none absolute top-0 left-0 h-full w-0 in-data-[mode=pattern]:opacity-45"
        >
          <span className="absolute top-0 -left-[5px] border-x-[5px] border-t-[7px] border-x-transparent border-t-(--wf-playhead)" />
          <span className="absolute top-0 left-0 h-full w-px bg-(--wf-playhead)" />
        </div>
      </div>
    </ContextActions>
  )
}
