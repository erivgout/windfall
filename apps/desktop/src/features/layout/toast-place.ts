import { logicalDelta, uiScaleFactor } from "@/lib/ui-scale"
import { useEffect, useState } from "react"

import { useEffectsUi } from "@/features/mixer/effects-ui"
import { useUiStore } from "@/lib/store/ui"

/*
 * Where toasts sit: at the bottom right, just above the status bar. Up at
 * the top they lay over the search box, the undo and redo buttons and the
 * playlist's toolbar. Down here the one thing in that corner is the mixer's
 * effects, when they are showing, and toasts move over to the left of them.
 */

/** Height of the status bar, which toasts sit above. */
const STATUS_BAR = 24
const GAP = 6

/**
 * How far from the right edge of the window toasts start, in pixels: past
 * every element marked `data-toast-clear` that reaches down to where they
 * sit. A panel puts that attribute on what toasts must not cover.
 */
export function toastInset(): number {
  const row = window.innerHeight - (STATUS_BAR + GAP) * uiScaleFactor()
  let inset = 0
  for (const element of document.querySelectorAll("[data-toast-clear]")) {
    const box = element.getBoundingClientRect()
    if (box.width === 0 || box.height === 0) continue
    // Only what lies in the row of the toasts and at the right.
    if (box.bottom < row - 4 || box.top > row) continue
    if (box.right < window.innerWidth - 2 * GAP) continue
    inset = Math.max(inset, window.innerWidth - box.left)
  }
  return logicalDelta(inset)
}

/**
 * The place of the toasts, as the `offset` of the toaster. It is measured
 * again when the window is resized and when the mixer or its effects are
 * shown, hidden or resized: those are what can be in that corner.
 */
export function useToastOffset(): { bottom: number; right: number } {
  const [inset, setInset] = useState(0)
  const mixer = useUiStore((state) => state.panels.mixer)
  const overlay = useUiStore((state) => state.centerOverlay)
  const layouts = useUiStore((state) => state.layouts)
  const uiScale = useUiStore((state) => state.uiScale)
  const inspector = useEffectsUi((state) => state.inspectorOpen)

  useEffect(() => {
    // After the panels have been laid out for this state.
    let frame = requestAnimationFrame(() => setInset(toastInset()))
    const onResize = () => {
      cancelAnimationFrame(frame)
      frame = requestAnimationFrame(() => setInset(toastInset()))
    }
    window.addEventListener("resize", onResize)
    return () => {
      cancelAnimationFrame(frame)
      window.removeEventListener("resize", onResize)
    }
  }, [mixer, overlay, layouts, inspector, uiScale])

  return { bottom: STATUS_BAR + GAP, right: inset + GAP }
}
