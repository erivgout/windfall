import { logicalDelta, logicalWheel } from "@/lib/ui-scale"
import {
  createPointerFrame,
  hitTestPoint,
  type PointerFrame,
  type TimeGridView,
} from "@/lib/canvas"
import { useProjectStore } from "@/lib/store/project"
import { useUiStore } from "@/lib/store/ui"

import type { PointerInput } from "./editor"
import { cursorFor } from "./intents"
import type { PianoRollSession } from "./session"
import { usePianoRollStore } from "./store"
import { readContext } from "./create-session"
import { buildGhostBatch } from "./ghosts"

const WHEEL_ZOOM = 0.0015
/** Dragging this close to an edge, or past it, scrolls the view along. */
const EDGE_PX = 10
/** A press that has not travelled this far does not scroll the view. */
const SCROLL_AFTER_PX = 6
const MAX_SCROLL_PX_PER_FRAME = 28

function toInput(
  frame: PointerFrame,
  event: MouseEvent,
  modifiers?: Partial<PointerInput>
): PointerInput {
  const point = frame.point(event)
  return {
    x: point.x,
    y: point.y,
    shift: event.shiftKey,
    ctrl: event.ctrlKey || event.metaKey,
    alt: event.altKey,
    ...modifiers,
  }
}

/**
 * Wheel navigation shared by the grid and the strips around it. `axes`
 * says which directions the strip under the pointer can scroll.
 */
export function handleWheel(
  session: PianoRollSession,
  event: WheelEvent,
  point: { x: number; y: number },
  axes: { time: boolean; rows: boolean }
): void {
  const view = session.view
  if (!view) return
  event.preventDefault()
  // Shift turns a vertical wheel into a horizontal one on some systems.
  // Preserve the roll's existing line/page step sizes at 100%.
  const wheel = logicalWheel(event, { line: 1, page: 1 })
  const delta = wheel.deltaY !== 0 ? wheel.deltaY : wheel.deltaX
  const zoom = event.ctrlKey || event.metaKey
  if (event.altKey || (zoom && event.shiftKey)) {
    if (axes.rows) session.zoomRows(point.y, delta < 0 ? 1.15 : 1 / 1.15)
    return
  }
  if (zoom) {
    if (axes.time) view.zoomTime(point.x, Math.exp(-delta * WHEEL_ZOOM))
    return
  }
  if (event.shiftKey || !axes.rows) {
    if (axes.time) view.panBy(delta, 0)
    return
  }
  view.panBy(axes.time ? wheel.deltaX : 0, wheel.deltaY)
}

function edgeSpeed(position: number, size: number): number {
  let over = 0
  if (position < EDGE_PX) over = position - EDGE_PX
  else if (position > size - EDGE_PX) over = position - (size - EDGE_PX)
  return Math.max(
    -MAX_SCROLL_PX_PER_FRAME,
    Math.min(MAX_SCROLL_PX_PER_FRAME, over * 0.4)
  )
}

/**
 * Turns mouse input on the grid canvas into editor gestures and view
 * navigation: left and right button for the tools, middle-drag to pan, the
 * wheel to scroll and zoom. Returns a function that detaches everything.
 */
export function attachGridInput(
  session: PianoRollSession,
  view: TimeGridView
): () => void {
  const { editor } = session
  const element = view.element
  // A drag is measured from where the grid was when it began, so a layout
  // change under a still pointer is never taken for a drag.
  const frame = createPointerFrame((event) => view.localPoint(event))
  let last: PointerInput | null = null
  let pointerInside = false
  let pressed: PointerInput | null = null
  let travelled = false
  let pan: { x: number; y: number } | null = null
  let scrollFrame = 0
  let stampCanceledByRightClick = false

  const canRefresh = () =>
    pointerInside || (frame.held && editor.hasPointerGesture)

  const showCursor = () => {
    element.style.cursor = pan
      ? "grabbing"
      : cursorFor(editor.hover?.intent ?? null)
  }
  const stopCursor = editor.subscribe((event) => {
    if (event === "hover") showCursor()
  })

  const stopAutoScroll = () => {
    if (scrollFrame !== 0) cancelAnimationFrame(scrollFrame)
    scrollFrame = 0
  }

  // While a drag is held at an edge the view keeps scrolling and the
  // gesture is fed the same pointer position, which now means a new place.
  const autoScroll = () => {
    scrollFrame = 0
    if (!editor.hasPointerGesture || !last) return
    if (usePianoRollStore.getState().tool === "zoom") return
    const { width, height } = view.viewport
    const dx = edgeSpeed(last.x, width)
    const dy = edgeSpeed(last.y, height)
    if (travelled && (dx !== 0 || dy !== 0)) {
      const before = view.viewport
      view.panBy(dx, dy)
      if (view.viewport !== before) editor.pointerMove(last)
    }
    scrollFrame = requestAnimationFrame(autoScroll)
  }

  const endGesture = (event: PointerEvent, commit: boolean) => {
    stopAutoScroll()
    if (pan) {
      pan = null
      showCursor()
    } else if (editor.hasPointerGesture) {
      if (commit) editor.pointerUp(toInput(frame, event))
      else editor.cancel()
    } else if (!commit) {
      editor.cancel()
    }
    frame.release()
    if (!pointerInside) editor.pointerLeave()
    if (element.hasPointerCapture(event.pointerId)) {
      element.releasePointerCapture(event.pointerId)
    }
  }

  const onPointerDown = (event: PointerEvent) => {
    if (event.pointerType === "touch") return
    pointerInside = true
    stampCanceledByRightClick = event.button === 2 && editor.hasStampChoice
    session.focusGrid()
    if (event.button === 1) {
      // Stops the browser's own middle-button scrolling.
      event.preventDefault()
      editor.cancel()
      pan = { x: event.clientX, y: event.clientY }
      element.setPointerCapture(event.pointerId)
      showCursor()
      return
    }
    if (event.button !== 0 && event.button !== 2) return
    frame.hold(event)
    last = toInput(frame, event)
    pressed = last
    travelled = false
    const preferences = usePianoRollStore.getState()
    const editing = session.editing
    if (editing && preferences.ghosts && preferences.editGhosts &&
      !editor.busy && !editor.hasStampChoice && !hitTestPoint(view.viewport, editor.items, last.x, last.y)) {
      const project = useProjectStore.getState().project
      const pattern = project.patterns.find((pattern) => pattern.id === editing.patternId)
      const lanes = pattern?.lanes.filter((lane) => lane.channel !== editing.channelId) ?? []
      const ghosts = buildGhostBatch(lanes, view.theme)
      const hit = ghosts ? hitTestPoint(view.viewport, ghosts, last.x, last.y) : null
      const source = hit && lanes.find((lane) => lane.notes.some((note) => note.id === hit.id))
      if (source && project.channels.some((channel) => channel.id === source.channel)) {
        const context = readContext(editing.patternId, source.channel)
        if (context) {
          session.keepViewportForLane(`${editing.patternId}:${source.channel}`)
          session.setEditing(editing.patternId, source.channel)
          editor.setContext(context)
          useUiStore.getState().selectChannel(source.channel)
        }
      }
    }
    editor.pointerDown(last, event.button === 0 ? "left" : "right")
    if (!editor.hasPointerGesture) {
      stopAutoScroll()
      frame.release()
      if (element.hasPointerCapture(event.pointerId)) {
        element.releasePointerCapture(event.pointerId)
      }
      return
    }
    element.setPointerCapture(event.pointerId)
    if (scrollFrame === 0) scrollFrame = requestAnimationFrame(autoScroll)
  }

  const onPointerMove = (event: PointerEvent) => {
    // Captured moves still target the canvas after the pointer goes outside it.
    const point = view.localPoint(event)
    pointerInside =
      point.x >= 0 &&
      point.x < view.viewport.width &&
      point.y >= 0 &&
      point.y < view.viewport.height
    if (pan) {
      view.panBy(
        logicalDelta(pan.x - event.clientX),
        logicalDelta(pan.y - event.clientY)
      )
      pan = { x: event.clientX, y: event.clientY }
      return
    }
    last = toInput(frame, event)
    if (!canRefresh()) {
      editor.pointerLeave()
      return
    }
    if (
      pressed &&
      Math.hypot(last.x - pressed.x, last.y - pressed.y) > SCROLL_AFTER_PX
    ) {
      travelled = true
    }
    editor.pointerMove(last)
  }

  const onPointerUp = (event: PointerEvent) => endGesture(event, true)
  const onPointerCancel = (event: PointerEvent) => endGesture(event, false)
  const onLostCapture = (event: PointerEvent) => {
    if (frame.held && editor.hasPointerGesture) endGesture(event, false)
  }

  const onPointerLeave = () => {
    pointerInside = false
    editor.pointerLeave()
  }

  // Right-click deletes in every tool but Select, where it opens the menu.
  const onContextMenu = (event: MouseEvent) => {
    if (stampCanceledByRightClick || editor.hasStampChoice) {
      stampCanceledByRightClick = false
      editor.cancel()
      event.preventDefault()
      event.stopPropagation()
      return
    }
    if (usePianoRollStore.getState().tool === "select") return
    event.preventDefault()
    event.stopPropagation()
  }

  const onMouseDown = (event: MouseEvent) => {
    if (event.button === 1) event.preventDefault()
  }

  const onWheel = (event: WheelEvent) => {
    handleWheel(session, event, frame.point(event), {
      time: true,
      rows: true,
    })
    if (last && canRefresh()) editor.pointerMove(last)
  }

  // Ctrl and Shift change what a drag does, also while the mouse is still.
  const onModifier = (event: KeyboardEvent) => {
    if (event.type === "keydown" && event.key === "Escape" && (editor.busy || editor.canRestoreZoom)) {
      event.preventDefault()
      stopAutoScroll()
      frame.release()
      editor.cancel()
      return
    }
    if (!last || !editor.busy || !canRefresh()) return
    if (!["Control", "Shift", "Alt", "Meta"].includes(event.key)) return
    last = {
      ...last,
      shift: event.shiftKey,
      ctrl: event.ctrlKey || event.metaKey,
      alt: event.altKey,
    }
    editor.pointerMove(last)
  }

  const onBlur = () => {
    stopAutoScroll()
    pan = null
    pointerInside = false
    frame.release()
    editor.cancel()
  }

  element.addEventListener("pointerdown", onPointerDown)
  element.addEventListener("pointermove", onPointerMove)
  element.addEventListener("pointerenter", onPointerMove)
  element.addEventListener("pointerup", onPointerUp)
  element.addEventListener("pointercancel", onPointerCancel)
  element.addEventListener("lostpointercapture", onLostCapture)
  element.addEventListener("pointerleave", onPointerLeave)
  element.addEventListener("contextmenu", onContextMenu)
  element.addEventListener("mousedown", onMouseDown)
  // Wheel must not be passive, or Ctrl+wheel would zoom the whole window.
  element.addEventListener("wheel", onWheel, { passive: false })
  window.addEventListener("keydown", onModifier)
  window.addEventListener("keyup", onModifier)
  window.addEventListener("blur", onBlur)

  return () => {
    editor.cancel()
    stopAutoScroll()
    stopCursor()
    element.removeEventListener("pointerdown", onPointerDown)
    element.removeEventListener("pointermove", onPointerMove)
    element.removeEventListener("pointerenter", onPointerMove)
    element.removeEventListener("pointerup", onPointerUp)
    element.removeEventListener("pointercancel", onPointerCancel)
    element.removeEventListener("lostpointercapture", onLostCapture)
    element.removeEventListener("pointerleave", onPointerLeave)
    element.removeEventListener("contextmenu", onContextMenu)
    element.removeEventListener("mousedown", onMouseDown)
    element.removeEventListener("wheel", onWheel)
    window.removeEventListener("keydown", onModifier)
    window.removeEventListener("keyup", onModifier)
    window.removeEventListener("blur", onBlur)
  }
}
