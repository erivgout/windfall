import { useRef, useState } from "react"

import type { Command } from "@/bindings"
import { useGesture } from "@/lib/store"

/**
 * Connects one of the kit's value controls to the project: spread the
 * result on a fader or knob. While the control is being moved it shows the
 * value under the pointer, not the stored one, so it never waits for the
 * backend to answer; every change of one drag lands in a single undo step.
 */
export function useGestureValue(
  stored: number,
  command: (value: number) => Command,
  clampValue: (value: number) => number
) {
  const gesture = useGesture()
  const [local, setLocal] = useState<number | null>(null)
  const moving = useRef(false)
  const inFlight = useRef(0)

  // The stored value takes over again once the drag is done and the
  // backend has answered every edit, so the control never jumps back to a
  // value from the middle of the drag.
  function settle() {
    if (!moving.current && inFlight.current === 0) setLocal(null)
  }

  return {
    value: local ?? stored,
    onGestureStart() {
      moving.current = true
      gesture.begin()
    },
    onValueChange(next: number) {
      const value = clampValue(next)
      setLocal(value)
      inFlight.current += 1
      void gesture.dispatch(command(value)).finally(() => {
        inFlight.current -= 1
        settle()
      })
    },
    onGestureEnd() {
      moving.current = false
      gesture.end()
      settle()
    },
  }
}
