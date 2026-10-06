import { useEffect, useRef } from "react"

import { TimeGridView, type TimeGridViewOptions } from "./time-grid-view"

export interface TimeGridCanvasProps {
  /** Read once per mount. Keep the object stable or the view is rebuilt. */
  readonly options: TimeGridViewOptions
  /**
   * Receives the view once its renderer exists. Drive the view through it;
   * nothing about the drawing goes through React state. Keep it stable.
   */
  readonly onReady: (view: TimeGridView) => void
  readonly onError?: (error: unknown) => void
  readonly className?: string
}

/** Mounts a `TimeGridView` in a div and destroys it on unmount. */
export function TimeGridCanvas({
  options,
  onReady,
  onError,
  className,
}: TimeGridCanvasProps) {
  const containerRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const container = containerRef.current
    if (!container) return
    let view: TimeGridView | null = null
    let cancelled = false
    TimeGridView.create(container, options).then(
      (created) => {
        // The effect can be cleaned up while the renderer is still being
        // created (strict mode does this on every mount).
        if (cancelled) {
          created.destroy()
          return
        }
        view = created
        onReady(created)
      },
      (error: unknown) => {
        if (!cancelled) onError?.(error)
      }
    )
    return () => {
      cancelled = true
      view?.destroy()
    }
  }, [options, onReady, onError])

  return <div ref={containerRef} className={className} />
}
