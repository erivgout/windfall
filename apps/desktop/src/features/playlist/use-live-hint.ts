import { useEffect, useMemo, useRef } from "react"

import { useHintStore } from "@/lib/store/hint"

/**
 * Like `useHint`, for a hint whose words change while the pointer is still
 * over the element: the grid's hint depends on the tool, and the tool can
 * be changed from the keyboard without the pointer moving.
 */
export function useLiveHint(text: string) {
  const latest = useRef(text)
  const shown = useRef<string | null>(null)

  useEffect(() => {
    latest.current = text
    if (shown.current === null) return
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

  return useMemo(
    () => ({
      onPointerEnter: () => {
        shown.current = latest.current
        useHintStore.setState({ text: latest.current })
      },
      onPointerLeave: () => {
        if (useHintStore.getState().text === shown.current) {
          useHintStore.setState({ text: null })
        }
        shown.current = null
      },
    }),
    []
  )
}
