import { useMemo, useRef } from "react"

import type { Command, DispatchResult } from "@/bindings"

import { dispatch } from "./project"

// Ids only need to differ from other gestures in flight, including ones from
// another window, so each window starts counting from a random place.
const base = Math.floor(Math.random() * 2 ** 31) * 2 ** 20
let count = 0

/** A new id that marks the edits of one drag as a single undo step. */
export function newGestureId(): number {
  count += 1
  return base + count
}

export type Gesture = {
  /** Call when the drag starts. */
  begin(): void
  /** Dispatches as part of the drag, or as its own undo step outside one. */
  dispatch(command: Command): Promise<DispatchResult | null>
  /** Call when the drag ends, so the next edit is a new undo step. */
  end(): void
}

/**
 * Groups the many edits of one drag into one undo step:
 *
 *     const gesture = useGesture()
 *     onPointerDown: gesture.begin()
 *     onPointerMove: gesture.dispatch({ type: "updateChannel", ... })
 *     onPointerUp:   gesture.end()
 */
export function useGesture(): Gesture {
  const id = useRef<number | undefined>(undefined)
  return useMemo(
    () => ({
      begin() {
        id.current = newGestureId()
      },
      dispatch(command) {
        return dispatch(command, id.current)
      },
      end() {
        id.current = undefined
      },
    }),
    []
  )
}
