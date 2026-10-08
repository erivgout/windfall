import { logicalDelta } from "@/lib/ui-scale"
import { useEffect, useRef } from "react"

import {
  ContextActions,
  contextSeparator,
  type ContextItem,
} from "@/components/context-actions"
import {
  deviceTransform,
  deviceX,
  rgbaToCss,
  tickToX,
  visibleTicks,
  withAlpha,
  xToTick,
} from "@/lib/canvas"
import { ignoresSnap } from "@/lib/edit-modifiers"
import { dispatch } from "@/lib/store/project"
import { seek, useTransportStore } from "@/lib/store/transport"
import { ticksPerBar, ticksPerBeat } from "@/lib/time"
import { TICKS_PER_STEP } from "@/lib/units"

import { CanvasLayer, type LayerSize } from "./canvas-layer"
import { useSession } from "./context"
import { lengthStepsAt } from "./edit-math"
import { handleWheel } from "./grid-input"
import { showHint } from "./hint"
import { VIEW_MENU } from "./menu"
import { fadePastEnd, shownLengthTicks } from "./overlays"
import type { PianoRollSession } from "./session"
import { snapRound, snapTicks } from "./snap"
import { usePianoRollStore } from "./store"

const HANDLE_REACH_PX = 6
const MIN_BAR_LABEL_PX = 46
const MIN_TICK_MARK_PX = 10
const MIN_BEAT_LABEL_PX = 64

const END_HINT =
  "Drag to change the pattern length by bars. Alt: by steps. Notes past the end do not play"
const RULER_HINT =
  "Click to move the playhead. Drag the marker at the end of the pattern to change its length"

const RULER_MENU: ContextItem[] = [
  ...VIEW_MENU,
  contextSeparator,
  "pattern.length16",
  "pattern.length32",
  "pattern.length48",
  "pattern.length64",
]

function paintRuler(
  ctx: CanvasRenderingContext2D,
  size: LayerSize,
  session: PianoRollSession
) {
  const view = session.view
  const context = session.editor.context
  if (!view || !context) return
  const { viewport, theme } = view
  const transform = deviceTransform({ ...viewport, dpr: size.dpr })
  const signature = context.pattern.signature
  const beat = ticksPerBeat(signature)
  const bar = ticksPerBar(signature)
  const px = viewport.pxPerTick
  const dpr = size.dpr
  const lw = transform.lineWidth
  const height = size.height

  let barsPerLabel = 1
  while (barsPerLabel * bar * px < MIN_BAR_LABEL_PX) barsPerLabel *= 2
  const showBeats = beat * px >= MIN_TICK_MARK_PX
  const showSteps =
    TICKS_PER_STEP * px >= MIN_TICK_MARK_PX && beat % TICKS_PER_STEP === 0
  const labelBeats = beat * px >= MIN_BEAT_LABEL_PX
  const unit = showSteps ? TICKS_PER_STEP : showBeats ? beat : bar

  ctx.font = `500 ${Math.round(10 * dpr)}px "Inter Variable", system-ui, sans-serif`
  ctx.textBaseline = "alphabetic"
  const baseline = Math.round(12 * dpr)
  const pad = Math.round(4 * dpr)
  const strongLine = rgbaToCss(theme.gridBar)
  const faintLine = rgbaToCss(theme.gridBeat)
  const barInk = rgbaToCss(withAlpha(theme.foreground, 0.9))
  const beatInk = rgbaToCss(withAlpha(theme.mutedForeground, 0.8))

  const range = visibleTicks(viewport)
  const first = Math.max(0, Math.floor(range.start / unit) * unit)
  for (let tick = first; tick <= range.end; tick += unit) {
    const x = deviceX(transform, tick)
    if (tick % bar === 0) {
      const index = tick / bar
      const labeled = index % barsPerLabel === 0
      ctx.fillStyle = strongLine
      const top = labeled ? 0 : Math.round(height * 0.55)
      ctx.fillRect(x, top, lw, height - top)
      if (labeled) {
        ctx.fillStyle = barInk
        ctx.fillText(String(index + 1), x + pad, baseline)
      }
    } else if (tick % beat === 0) {
      ctx.fillStyle = faintLine
      const mark = Math.round(7 * dpr)
      ctx.fillRect(x, height - mark, lw, mark)
      if (labelBeats) {
        const inBar = Math.floor((tick % bar) / beat)
        ctx.fillStyle = beatInk
        ctx.fillText(
          `${Math.floor(tick / bar) + 1}.${inBar + 1}`,
          x + pad,
          baseline
        )
      }
    } else {
      ctx.fillStyle = faintLine
      const mark = Math.round(3 * dpr)
      ctx.fillRect(x, height - mark, lw, mark)
    }
  }

  // The length of the pattern: a strip along the bottom up to the marker
  // that can be dragged, and everything after it dimmed.
  const end = shownLengthTicks(session)
  const endX = deviceX(transform, end)
  const startX = Math.max(0, deviceX(transform, 0))
  const brand = rgbaToCss(theme.item)
  if (endX < size.width) {
    const from = Math.max(0, endX)
    fadePastEnd(ctx, theme, from, size.width - from, height)
  }
  const strip = Math.round(2 * dpr)
  if (endX > startX) {
    ctx.fillStyle = rgbaToCss(withAlpha(theme.item, 0.75))
    ctx.fillRect(startX, height - strip, endX - startX, strip)
  }
  if (endX >= -10 * dpr && endX <= size.width + 10 * dpr) {
    const flag = Math.round(8 * dpr)
    ctx.fillStyle = brand
    ctx.fillRect(endX - lw, 0, 2 * lw, height)
    ctx.beginPath()
    ctx.moveTo(endX, 0)
    ctx.lineTo(endX - flag, 0)
    ctx.lineTo(endX, flag)
    ctx.closePath()
    ctx.fill()
  }

  if (session.playhead !== null) {
    const x = deviceX(transform, session.playhead)
    const half = Math.round(4 * dpr)
    ctx.fillStyle = rgbaToCss(theme.playhead)
    ctx.beginPath()
    ctx.moveTo(x - half, 0)
    ctx.lineTo(x + half + lw, 0)
    ctx.lineTo(x + lw / 2, Math.round(7 * dpr))
    ctx.closePath()
    ctx.fill()
    ctx.fillRect(x, 0, lw, height)
  }
}

/**
 * The time ruler above the grid: bars and beats, the playhead, and the end
 * of the pattern as a marker that can be dragged to make it longer or
 * shorter.
 */
export function Ruler() {
  const session = useSession()
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const dragging = useRef(false)

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return
    const layer = new CanvasLayer(canvas, (ctx, size) =>
      paintRuler(ctx, size, session)
    )
    const stop = [
      session.onView(() => layer.invalidate()),
      session.onPlayhead(() => layer.invalidate()),
    ]
    const onWheel = (event: WheelEvent) => {
      const bounds = canvas.getBoundingClientRect()
      handleWheel(
        session,
        event,
        { x: logicalDelta(event.clientX - bounds.left), y: 0 },
        { time: true, rows: false }
      )
    }
    canvas.addEventListener("wheel", onWheel, { passive: false })
    return () => {
      for (const off of stop) off()
      canvas.removeEventListener("wheel", onWheel)
      layer.destroy()
    }
  }, [session])

  function localX(event: React.PointerEvent<HTMLCanvasElement>): number {
    return logicalDelta(
      event.clientX - event.currentTarget.getBoundingClientRect().left
    )
  }

  function nearEnd(x: number): boolean {
    const view = session.view
    if (!view) return false
    const endX = tickToX(view.viewport, shownLengthTicks(session))
    return Math.abs(x - endX) <= HANDLE_REACH_PX
  }

  function previewLength(event: React.PointerEvent<HTMLCanvasElement>) {
    const view = session.view
    const context = session.editor.context
    if (!view || !context) return
    const steps = lengthStepsAt(
      xToTick(view.viewport, localX(event)),
      context.pattern.signature,
      // By steps is this drag with the snap let go.
      ignoresSnap({ alt: event.altKey })
    )
    session.setLengthPreview(steps * TICKS_PER_STEP)
  }

  function endPreview() {
    session.setLengthPreview(null)
  }

  function onPointerDown(event: React.PointerEvent<HTMLCanvasElement>) {
    const view = session.view
    const context = session.editor.context
    if (event.button !== 0 || !view || !context) return
    const x = localX(event)
    if (nearEnd(x)) {
      dragging.current = true
      event.currentTarget.setPointerCapture(event.pointerId)
      return
    }
    // The song's position belongs to the playlist, so only the pattern's moves.
    if (useTransportStore.getState().mode !== "pattern") return
    const snap = snapTicks(
      usePianoRollStore.getState().snap,
      context.pattern.signature
    )
    const last = context.pattern.lengthSteps * TICKS_PER_STEP - 1
    const tick = snapRound(xToTick(view.viewport, x), snap)
    void seek(Math.min(last, Math.max(0, tick)))
  }

  function onPointerMove(event: React.PointerEvent<HTMLCanvasElement>) {
    if (dragging.current) {
      previewLength(event)
      return
    }
    const near = nearEnd(localX(event))
    event.currentTarget.style.cursor = near ? "ew-resize" : "default"
    showHint(near ? END_HINT : RULER_HINT)
  }

  function onPointerUp(event: React.PointerEvent<HTMLCanvasElement>) {
    if (!dragging.current) return
    dragging.current = false
    previewLength(event)
    const context = session.editor.context
    const preview = session.lengthPreview
    if (!context || preview === null) {
      endPreview()
      return
    }
    const lengthSteps = Math.round(preview / TICKS_PER_STEP)
    if (lengthSteps === context.pattern.lengthSteps) {
      endPreview()
      return
    }
    // The preview stays until the project answers, so the marker does not
    // jump back for a frame.
    void dispatch({
      type: "updatePattern",
      id: context.pattern.id,
      patch: { lengthSteps },
    }).finally(endPreview)
  }

  function onPointerCancel() {
    if (!dragging.current) return
    dragging.current = false
    endPreview()
  }

  return (
    <ContextActions items={RULER_MENU}>
      <canvas
        ref={canvasRef}
        aria-label="Time ruler. Drag the marker at the end of the pattern to change its length"
        className="block h-full w-full touch-none"
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerCancel={onPointerCancel}
        onPointerLeave={() => showHint(null)}
      />
    </ContextActions>
  )
}
