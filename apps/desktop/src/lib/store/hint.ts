import { useEffect, useMemo, useRef } from "react"
import { create } from "zustand"

type HintState = {
  text: string | null
  /**
   * Something the user should know about what they just did, which takes
   * the hint's place for a while. Whoever sets it takes it away again.
   */
  notice: string | null
}

/** The line of help the status bar shows for the control under the pointer. */
export const useHintStore = create<HintState>(() => ({
  text: null,
  notice: null,
}))

/**
 * Returns handlers to spread on a control so the status bar explains it
 * while it is hovered or focused:
 *
 *     <button {...useHint("Drag to change the tempo")} />
 *
 * The text may hold a value. While the control is hovered or focused the
 * status bar follows the text as it changes, so a knob being turned reads
 * out its level there.
 */
export function useHint(text: string) {
  const latest = useRef(text)
  /** What this control has put in the status bar, if it is still there. */
  const shown = useRef<string | null>(null)
  const holders = useRef({ pointer: false, focus: false })

  useEffect(() => {
    latest.current = text
    if (shown.current === null || shown.current === text) return
    if (useHintStore.getState().text === shown.current) {
      useHintStore.setState({ text })
    }
    shown.current = text
  }, [text])

  useEffect(
    () => () => {
      if (
        shown.current !== null &&
        useHintStore.getState().text === shown.current
      ) {
        useHintStore.setState({ text: null })
      }
    },
    []
  )

  return useMemo(() => {
    const update = () => {
      if (holders.current.pointer || holders.current.focus) {
        shown.current = latest.current
        useHintStore.setState({ text: latest.current })
      } else if (shown.current !== null) {
        if (useHintStore.getState().text === shown.current) {
          useHintStore.setState({ text: null })
        }
        shown.current = null
      }
    }
    return {
      onPointerEnter: () => {
        holders.current.pointer = true
        update()
      },
      onPointerLeave: () => {
        holders.current.pointer = false
        update()
      },
      onFocus: () => {
        holders.current.focus = true
        update()
      },
      onBlur: () => {
        holders.current.focus = false
        update()
      },
    }
  }, [])
}
