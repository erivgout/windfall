import {
  deviceY,
  rowToKey,
  visibleRows,
  type OverlayPainter,
} from "@/lib/canvas"

import type { PianoRollSession } from "../session"
import { isInScale, type ScaleHighlight } from "./model"

/** A light veil keeps out-of-scale notes legible without touching their batch. */
export function scaleHighlightPainter(choice: ScaleHighlight): OverlayPainter {
  return (ctx, { viewport, transform }) => {
    if (choice.scale === "off") return
    const rows = visibleRows(viewport, 128)
    ctx.save()
    ctx.fillStyle = "rgba(0, 0, 0, 0.18)"
    for (let row = rows.first; row < rows.last; row++) {
      if (isInScale(rowToKey(row), choice)) continue
      const top = deviceY(transform, row)
      const bottom = deviceY(transform, row + 1)
      ctx.fillRect(0, top, transform.widthDev, bottom - top)
    }
    ctx.restore()
  }
}

/** Follow view attachment/replacement without adding a workspace dependency. */
export function attachScaleHighlight(
  session: PianoRollSession,
  choice: ScaleHighlight
): () => void {
  if (choice.scale === "off") return () => {}
  const painter = scaleHighlightPainter(choice)
  let view: PianoRollSession["view"] = null
  let remove: (() => void) | null = null
  const attach = () => {
    if (view === session.view) return
    remove?.()
    view = session.view
    remove = view?.addOverlayPainter(painter) ?? null
  }
  attach()
  const stop = session.onView(attach)
  return () => {
    stop()
    remove?.()
  }
}
