import { useEffect, useMemo, useRef } from "react"

import { useHintStore } from "@/lib/store/hint"

/**
 * Like `useHint`, for a hint that contains a value: while the control is
 * hovered or focused the status bar follows the text as it changes, so a
 * knob being turned reads out its level there.
 */
export function useLiveHint(text: string) {
  const shown = useRef<string | null>(null)
  const holders = useRef({ pointer: false, focus: false })

  useEffect(() => {
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
    const update = (latest: string) => {
      const active = holders.current.pointer || holders.current.focus
      if (active) {
        shown.current = latest
        useHintStore.setState({ text: latest })
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
        update(text)
      },
      onPointerLeave: () => {
        holders.current.pointer = false
        update(text)
      },
      onFocus: () => {
        holders.current.focus = true
        update(text)
      },
      onBlur: () => {
        holders.current.focus = false
        update(text)
      },
    }
  }, [text])
}
