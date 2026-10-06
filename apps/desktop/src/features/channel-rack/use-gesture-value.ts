import { useLayoutEffect, useMemo, useRef, useState } from "react"

import type { Command, DispatchResult } from "@/bindings"
import { useGesture } from "@/lib/store/gesture"

type Send<T> = (
  value: T,
  dispatch: (command: Command) => Promise<DispatchResult | null>
) => Promise<unknown>

/**
 * Connects a kit control to the project. While the control is being moved it
 * shows its own value, so it never waits for the backend, and every change
 * of one drag is sent under one gesture id, so the drag is one undo step.
 *
 *     const volume = useGestureValue(channel.volume, (value, dispatch) =>
 *       dispatch({ type: "updateChannel", id, patch: { volume: value } }))
 *     <Knob {...volume} />
 */
export function useGestureValue<T>(stored: T, send: Send<T>) {
  const gesture = useGesture()
  const [local, setLocal] = useState<{ value: T } | null>(null)
  const state = useRef({
    active: false,
    pending: Promise.resolve() as Promise<unknown>,
    send,
  })
  // The handlers below are created once, so they reach the newest `send`
  // through the ref.
  useLayoutEffect(() => {
    state.current.send = send
  })

  const handlers = useMemo(
    () => ({
      onGestureStart() {
        state.current.active = true
        gesture.begin()
      },
      onValueChange(value: T) {
        setLocal({ value })
        state.current.pending = state.current.send(value, gesture.dispatch)
      },
      onGestureEnd() {
        state.current.active = false
        gesture.end()
        // Keep showing the dragged value until the project has caught up,
        // so the control does not jump back for a frame.
        const last = state.current.pending
        void last.finally(() => {
          if (!state.current.active && state.current.pending === last) {
            setLocal(null)
          }
        })
      },
    }),
    [gesture]
  )

  return { value: local ? local.value : stored, ...handlers }
}
