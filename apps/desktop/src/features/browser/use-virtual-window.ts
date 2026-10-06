import { useCallback, useLayoutEffect, useRef, useState } from "react"
import { flushSync } from "react-dom"

/** Every row is this tall, which is what makes the list cheap to window. */
export const ROW_HEIGHT = 22

// Rows drawn beyond each edge, for the frames where the compositor has
// scrolled further than the page has heard about.
const OVERSCAN = 12
// Used when the list has no measurable height yet, as in tests.
const FALLBACK_HEIGHT = 440

type Range = { first: number; fit: number }

/**
 * Works out which rows of a long list are on screen. Rows have a fixed
 * height, so the answer is arithmetic on the scroll position and nothing has
 * to be measured per row. A folder with 10,000 files draws a few dozen rows.
 */
export function useVirtualWindow(count: number) {
  const scrollRef = useRef<HTMLDivElement | null>(null)
  const [range, setRange] = useState<Range>({
    first: 0,
    fit: Math.ceil(FALLBACK_HEIGHT / ROW_HEIGHT),
  })

  const measure = useCallback(() => {
    const element = scrollRef.current
    if (!element) return
    const first = Math.floor(element.scrollTop / ROW_HEIGHT)
    const fit = Math.ceil(
      (element.clientHeight || FALLBACK_HEIGHT) / ROW_HEIGHT
    )
    setRange((previous) =>
      previous.first === first && previous.fit === fit
        ? previous
        : { first, fit }
    )
  }, [])

  useLayoutEffect(() => {
    const element = scrollRef.current
    if (!element) return undefined
    const observer = new ResizeObserver(measure)
    observer.observe(element)
    return () => observer.disconnect()
  }, [measure])

  // React would draw the new rows a frame after the scroll, which shows as
  // a blank strip when scrolling fast. Scroll events arrive right before a
  // frame is painted, so rendering inside one keeps the rows in step.
  const onScroll = useCallback(() => flushSync(measure), [measure])

  /** Scrolls just far enough to bring a row fully into view. */
  const reveal = useCallback(
    (index: number) => {
      const element = scrollRef.current
      if (!element || index < 0) return
      const top = index * ROW_HEIGHT
      const height = element.clientHeight || FALLBACK_HEIGHT
      if (top < element.scrollTop) {
        element.scrollTop = top
      } else if (top + ROW_HEIGHT > element.scrollTop + height) {
        element.scrollTop = top + ROW_HEIGHT - height
      }
      measure()
    },
    [measure]
  )

  const first = Math.min(range.first, Math.max(0, count - 1))
  return {
    scrollRef,
    start: Math.max(0, first - OVERSCAN),
    end: Math.min(count, first + range.fit + OVERSCAN),
    /** The most rows the window ever holds. */
    capacity: range.fit + 2 * OVERSCAN,
    /** How many rows fit on screen, for Page Up and Page Down. */
    pageSize: Math.max(1, range.fit - 1),
    measure,
    onScroll,
    reveal,
  }
}
