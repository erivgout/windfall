import { wheelInput, type GridMetrics } from "./metrics"
import type { PlaylistSession, PointerInput } from "./session"
import { usePlaylistStore } from "./store"

type Binding = {
  /** Converts a pointer position to CSS pixels inside the grid. */
  localPoint(event: { clientX: number; clientY: number }): {
    x: number
    y: number
  }
  /** Gives the grid the keyboard, so the arrow keys and Delete reach it. */
  focus(): void
}

function inputOf(event: PointerEvent, binding: Binding): PointerInput {
  return {
    ...binding.localPoint(event),
    button: event.button,
    shift: event.shiftKey,
    mod: event.ctrlKey || event.metaKey,
    alt: event.altKey,
  }
}

/**
 * Feeds the canvas's pointer and wheel events to the session. Returns a
 * function that stops.
 */
export function attachPointer(
  element: HTMLElement,
  session: PlaylistSession,
  metrics: GridMetrics,
  binding: Binding
): () => void {
  const onPointerDown = (event: PointerEvent) => {
    binding.focus()
    // Stops the browser's own middle-button scrolling.
    if (event.button === 1) event.preventDefault()
    if (session.pointerDown(inputOf(event, binding))) {
      element.setPointerCapture(event.pointerId)
    }
  }
  const onPointerMove = (event: PointerEvent) => {
    session.pointerMove(inputOf(event, binding))
  }
  const onPointerUp = (event: PointerEvent) => {
    if (element.hasPointerCapture(event.pointerId)) {
      element.releasePointerCapture(event.pointerId)
    }
    void session.pointerUp(inputOf(event, binding))
  }
  const onPointerCancel = () => session.cancel()
  const onMouseDown = (event: MouseEvent) => {
    if (event.button === 1) event.preventDefault()
  }
  const onContextMenu = (event: MouseEvent) => {
    // The right button deletes in every tool but Select, where the event
    // goes on to open the menu.
    if (usePlaylistStore.getState().tool === "select") return
    event.preventDefault()
    event.stopPropagation()
  }
  const onWheel = (event: WheelEvent) => {
    event.preventDefault()
    metrics.wheel(wheelInput(event, binding.localPoint(event)))
  }

  element.addEventListener("pointerdown", onPointerDown)
  element.addEventListener("pointermove", onPointerMove)
  element.addEventListener("pointerup", onPointerUp)
  element.addEventListener("pointercancel", onPointerCancel)
  element.addEventListener("mousedown", onMouseDown)
  element.addEventListener("contextmenu", onContextMenu)
  // Not passive, or Ctrl+wheel would zoom the whole window.
  element.addEventListener("wheel", onWheel, { passive: false })
  return () => {
    element.removeEventListener("pointerdown", onPointerDown)
    element.removeEventListener("pointermove", onPointerMove)
    element.removeEventListener("pointerup", onPointerUp)
    element.removeEventListener("pointercancel", onPointerCancel)
    element.removeEventListener("mousedown", onMouseDown)
    element.removeEventListener("contextmenu", onContextMenu)
    element.removeEventListener("wheel", onWheel)
  }
}
