import { uiScaleFactor } from "@/lib/ui-scale"

type ClientPoint = { clientX: number; clientY: number }
type LocalPoint = { x: number; y: number }

/**
 * Where the pointer is inside a grid, in a frame that stays put for as long
 * as a button is held.
 *
 * A press can change the layout around the grid (a strip appears, a toolbar
 * wraps), which moves the grid under a pointer that has not moved. Measured
 * against the grid's new place that would read as a drag. So the grid's
 * place is taken once, at the press, and every position until the release
 * is measured from there.
 */
export type PointerFrame = {
  /** A button went down: keep the frame the grid is in now. */
  hold(event: ClientPoint): void
  /** The button is up: follow the grid's place again. */
  release(): void
  readonly held: boolean
  point(event: ClientPoint): LocalPoint
}

export function createPointerFrame(
  localPoint: (event: ClientPoint) => LocalPoint
): PointerFrame {
  let origin: LocalPoint | null = null
  let scale = 1
  return {
    hold(event) {
      const local = localPoint(event)
      scale = uiScaleFactor()
      origin = {
        x: local.x - event.clientX / scale,
        y: local.y - event.clientY / scale,
      }
    },
    release() {
      origin = null
    },
    get held() {
      return origin !== null
    },
    point(event) {
      return origin
        ? {
            x: event.clientX / scale + origin.x,
            y: event.clientY / scale + origin.y,
          }
        : localPoint(event)
    },
  }
}
