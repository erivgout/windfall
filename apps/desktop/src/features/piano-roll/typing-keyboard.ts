import { create } from "zustand"

import type { ChannelId } from "@/bindings"
import { actionForEvent } from "@/lib/actions"
import { shortcutAllowed } from "@/lib/actions/keymap"
import { onProjectReplaced } from "@/lib/store/replaced"
import { DEFAULT_VELOCITY } from "@/lib/units"

import { auditionOff, auditionOn } from "./audition"

export const NOTE_OFFSETS: Readonly<Record<string, number>> = {
  a: 0,
  w: 1,
  s: 2,
  e: 3,
  d: 4,
  f: 5,
  t: 6,
  g: 7,
  y: 8,
  h: 9,
  u: 10,
  j: 11,
  k: 12,
  o: 13,
  l: 14,
  p: 15,
}

type TypingState = {
  enabled: boolean
  /** MIDI C played by A; independent of the note grid's scroll position. */
  baseKey: number
  setEnabled(enabled: boolean): void
  shiftOctave(direction: number): void
}

/** Session-only audition settings. Never persisted or written to the pattern. */
export const useTypingKeyboardStore = create<TypingState>((set) => ({
  enabled: false,
  baseKey: 60,
  setEnabled: (enabled) => set({ enabled }),
  shiftOctave: (direction) =>
    set((state) => ({
      baseKey: Math.max(24, Math.min(96, state.baseKey + direction * 12)),
    })),
}))

onProjectReplaced(() =>
  useTypingKeyboardStore.setState({ enabled: false, baseKey: 60 })
)

/** Owns physical key lifetimes, retaining each pitch even if the octave changes. */
export function attachTypingKeyboard(
  root: HTMLElement,
  channel: ChannelId
): () => void {
  const held = new Map<string, number>()
  const identity = (event: KeyboardEvent) =>
    event.code || event.key.toLowerCase()
  const release = () => {
    for (const key of held.values()) auditionOff(channel, key)
    held.clear()
  }
  const onKeyDown = (event: KeyboardEvent) => {
    const state = useTypingKeyboardStore.getState()
    if (!state.enabled || event.defaultPrevented || event.isComposing) return
    if (!root.contains(document.activeElement)) return
    // Fields and overlays keep their normal keyboard interaction.
    if (!shortcutAllowed(event.target, "A")) return
    // Let the registered toggle reach the keymap, including while Typing is on.
    const action = actionForEvent(event)
    if (
      action?.id === "pianoRoll.typing" ||
      action?.id === "pianoRoll.stepEntry"
    ) return
    const letter = event.key.toLowerCase()
    const offset = NOTE_OFFSETS[letter]
    const octaveKey = letter === "z" || letter === "x"
    // Unbound navigation keys (such as Tab) still move keyboard focus.
    if (typeof offset !== "number" && !octaveKey && !action) return

    event.preventDefault()
    event.stopPropagation()
    if (event.repeat || event.ctrlKey || event.metaKey || event.altKey) return
    if (octaveKey) {
      state.shiftOctave(letter === "z" ? -1 : 1)
      return
    }
    const id = identity(event)
    if (typeof offset !== "number" || held.has(id)) return
    const key = state.baseKey + offset
    held.set(id, key)
    auditionOn(channel, key, DEFAULT_VELOCITY)
  }
  const onKeyUp = (event: KeyboardEvent) => {
    const id = identity(event)
    const key = held.get(id)
    if (key === undefined) return
    held.delete(id)
    auditionOff(channel, key)
    event.preventDefault()
    event.stopPropagation()
  }
  const stop = useTypingKeyboardStore.subscribe((state) => {
    if (!state.enabled) release()
  })
  root.addEventListener("keydown", onKeyDown, true)
  root.addEventListener("focusout", release)
  window.addEventListener("keyup", onKeyUp, true)
  window.addEventListener("blur", release)
  return () => {
    release()
    stop()
    root.removeEventListener("keydown", onKeyDown, true)
    root.removeEventListener("focusout", release)
    window.removeEventListener("keyup", onKeyUp, true)
    window.removeEventListener("blur", release)
  }
}
