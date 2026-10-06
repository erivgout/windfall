import { useMemo } from "react"
import { create } from "zustand"

type HintState = { text: string | null }

/** The line of help the status bar shows for the control under the pointer. */
export const useHintStore = create<HintState>(() => ({ text: null }))

function show(text: string) {
  useHintStore.setState({ text })
}

function hide(text: string) {
  if (useHintStore.getState().text === text) {
    useHintStore.setState({ text: null })
  }
}

/**
 * Returns handlers to spread on a control so the status bar explains it
 * while it is hovered or focused:
 *
 *     <button {...useHint("Drag to change the tempo")} />
 */
export function useHint(text: string) {
  return useMemo(
    () => ({
      onPointerEnter: () => show(text),
      onPointerLeave: () => hide(text),
      onFocus: () => show(text),
      onBlur: () => hide(text),
    }),
    [text]
  )
}
