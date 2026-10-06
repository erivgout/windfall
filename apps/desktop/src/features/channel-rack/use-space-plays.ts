import { useMemo, type KeyboardEvent } from "react"

import { currentKeymap, runAction } from "@/lib/actions"
import { isTypingTarget } from "@/lib/actions/keymap"

/*
 * The rack's rule for the keyboard: Space always does what the keymap says
 * (play or stop), whatever is focused, and Enter presses the focused step,
 * lamp or button. Without this a focused step would take Space for itself
 * and the transport would seem to ignore the key after every click.
 */

function isPlainSpace(event: KeyboardEvent): boolean {
  return (
    (event.key === " " || event.code === "Space") &&
    !event.ctrlKey &&
    !event.altKey &&
    !event.metaKey &&
    !event.shiftKey
  )
}

/** False for typing, and for menus and dialogs that only render through us. */
function ownsEvent(event: KeyboardEvent): boolean {
  return (
    event.target instanceof Element &&
    event.currentTarget.contains(event.target) &&
    !isTypingTarget(event.target)
  )
}

/** Handlers for the root of the rack. They run before the focused control's. */
export function useSpacePlays() {
  return useMemo(
    () => ({
      onKeyDownCapture(event: KeyboardEvent) {
        if (!isPlainSpace(event) || !ownsEvent(event)) return
        const action = currentKeymap().byChord.get("Space")
        if (action === undefined) return
        event.preventDefault()
        event.stopPropagation()
        if (!event.repeat) void runAction(action)
      },
      // A button clicks itself when Space comes back up, so that is held
      // back as well.
      onKeyUpCapture(event: KeyboardEvent) {
        if (!isPlainSpace(event) || !ownsEvent(event)) return
        if (currentKeymap().byChord.get("Space") === undefined) return
        event.preventDefault()
        event.stopPropagation()
      },
    }),
    []
  )
}
