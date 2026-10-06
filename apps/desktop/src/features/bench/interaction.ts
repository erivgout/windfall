import {
  queryRect,
  snapTick,
  xToTick,
  yToRow,
  type TimeGridView,
} from "@/lib/canvas"

import { TICKS_PER_STEP } from "./generate-notes"
import { ROW_COUNT, type BenchModel } from "./model"

type Gesture =
  | { readonly kind: "idle" }
  | {
      readonly kind: "marquee"
      readonly tick: number
      readonly row: number
    }
  | {
      readonly kind: "move"
      readonly tick: number
      readonly row: number
      /** Row limits that keep every selected note on the keyboard. */
      readonly minRows: number
      readonly maxRows: number
    }

const PLAYHEAD_TICKS_PER_MS = (140 * 960) / 60_000

/**
 * Mouse and keyboard handling for trying the page by hand: wheel scrolls,
 * Ctrl+wheel zooms time, Alt+wheel zooms rows, dragging empty space
 * selects with a marquee, dragging a note moves the selection, Space
 * starts and stops a playhead, Escape clears the selection.
 *
 * The real piano roll will route all of this through the action registry
 * and the store. This is only here so the renderer can be judged by feel.
 */
export function attachInteraction(
  view: TimeGridView,
  model: BenchModel
): () => void {
  const element = view.element
  let gesture: Gesture = { kind: "idle" }
  let playing = false
  let playFrame = 0

  const onWheel = (event: WheelEvent): void => {
    event.preventDefault()
    const point = view.localPoint(event)
    if (event.ctrlKey || event.metaKey) {
      view.zoomTime(point.x, Math.exp(-event.deltaY * 0.0015))
    } else if (event.altKey) {
      view.zoomRows(point.y, Math.exp(-event.deltaY * 0.0015))
    } else if (event.shiftKey) {
      view.panBy(event.deltaY + event.deltaX, 0)
    } else {
      view.panBy(event.deltaX, event.deltaY)
    }
  }

  const onPointerDown = (event: PointerEvent): void => {
    if (event.button !== 0) return
    const point = view.localPoint(event)
    const batch = model.items.batch
    const tick = xToTick(view.viewport, point.x)
    const row = yToRow(view.viewport, point.y)
    const hit = view.hitTest(point.x, point.y)
    element.setPointerCapture(event.pointerId)

    if (!hit) {
      if (!event.shiftKey) batch.clearSelection()
      view.invalidate("base")
      gesture = { kind: "marquee", tick, row }
      return
    }
    if (!batch.isSelected(hit.index)) {
      if (!event.shiftKey) batch.clearSelection()
      batch.setSelected(hit.index, true)
      view.invalidate("base")
    }
    let top = ROW_COUNT
    let bottom = 0
    for (const index of batch.selectedIndices()) {
      top = Math.min(top, batch.row(index))
      bottom = Math.max(bottom, batch.row(index))
    }
    gesture = {
      kind: "move",
      tick,
      row: Math.floor(row),
      minRows: -top,
      maxRows: ROW_COUNT - 1 - bottom,
    }
  }

  const moveOffset = (
    move: Extract<Gesture, { kind: "move" }>,
    x: number,
    y: number
  ): { ticks: number; rows: number } => ({
    ticks: snapTick(xToTick(view.viewport, x) - move.tick, TICKS_PER_STEP),
    rows: Math.min(
      move.maxRows,
      Math.max(move.minRows, Math.floor(yToRow(view.viewport, y)) - move.row)
    ),
  })

  const onPointerMove = (event: PointerEvent): void => {
    const point = view.localPoint(event)
    switch (gesture.kind) {
      case "idle": {
        const hit = view.hitTest(point.x, point.y)
        element.style.cursor = !hit
          ? "default"
          : hit.part === "body"
            ? "move"
            : "ew-resize"
        return
      }
      case "marquee": {
        const tick = xToTick(view.viewport, point.x)
        const row = yToRow(view.viewport, point.y)
        view.setMarquee({
          tick0: gesture.tick,
          row0: gesture.row,
          tick1: tick,
          row1: row,
        })
        model.items.batch.setSelection(
          queryRect(
            model.items,
            Math.min(gesture.tick, tick),
            Math.max(gesture.tick, tick),
            Math.min(gesture.row, row),
            Math.max(gesture.row, row)
          )
        )
        view.invalidate("base")
        return
      }
      case "move": {
        const offset = moveOffset(gesture, point.x, point.y)
        view.setDragOffset(offset.ticks, offset.rows)
        return
      }
      default: {
        const _exhaustive: never = gesture
        return _exhaustive
      }
    }
  }

  const onPointerUp = (event: PointerEvent): void => {
    if (gesture.kind === "move") {
      const point = view.localPoint(event)
      const offset = moveOffset(gesture, point.x, point.y)
      view.setDragOffset(0, 0)
      model.moveSelected(offset.ticks, offset.rows)
    }
    view.setMarquee(null)
    gesture = { kind: "idle" }
    if (element.hasPointerCapture(event.pointerId)) {
      element.releasePointerCapture(event.pointerId)
    }
  }

  const stopPlayhead = (): void => {
    playing = false
    cancelAnimationFrame(playFrame)
    view.setPlayhead(null)
  }

  const startPlayhead = (): void => {
    playing = true
    const origin = view.viewport.scrollTick
    const started = performance.now()
    const tick = (): void => {
      view.setPlayhead(
        origin + (performance.now() - started) * PLAYHEAD_TICKS_PER_MS
      )
      playFrame = requestAnimationFrame(tick)
    }
    playFrame = requestAnimationFrame(tick)
  }

  const onKeyDown = (event: KeyboardEvent): void => {
    if (event.code === "Space") {
      event.preventDefault()
      if (playing) stopPlayhead()
      else startPlayhead()
    } else if (event.code === "Escape") {
      model.items.batch.clearSelection()
      view.invalidate("base")
    }
  }

  // Wheel must not be passive, or the page would zoom on Ctrl+wheel.
  element.addEventListener("wheel", onWheel, { passive: false })
  element.addEventListener("pointerdown", onPointerDown)
  element.addEventListener("pointermove", onPointerMove)
  element.addEventListener("pointerup", onPointerUp)
  element.addEventListener("pointercancel", onPointerUp)
  window.addEventListener("keydown", onKeyDown)

  return () => {
    stopPlayhead()
    element.removeEventListener("wheel", onWheel)
    element.removeEventListener("pointerdown", onPointerDown)
    element.removeEventListener("pointermove", onPointerMove)
    element.removeEventListener("pointerup", onPointerUp)
    element.removeEventListener("pointercancel", onPointerUp)
    window.removeEventListener("keydown", onKeyDown)
  }
}
