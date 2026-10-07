import { useEffect, useRef, type RefObject } from "react"

import {
  observeCanvas,
  resolveColors,
  subscribeTheme,
  type CanvasSize,
} from "@/components/audio"

export type DisplayPainter<Name extends string> = (
  context: CanvasRenderingContext2D,
  size: CanvasSize,
  colors: Record<Name, string>
) => void

/**
 * A canvas that draws a curve display. `paint` runs after a render, a
 * resize and a theme change, at most once per animation frame, with the
 * context already scaled to CSS pixels and `colors` resolved from the
 * theme's CSS variables. Keep `colors` the same object between renders.
 */
export function useDisplayCanvas<Name extends string>(
  colors: Record<Name, string>,
  paint: DisplayPainter<Name>
): RefObject<HTMLCanvasElement | null> {
  const canvas = useRef<HTMLCanvasElement>(null)
  const painter = useRef(paint)
  const request = useRef<() => void>(() => {})

  useEffect(() => {
    const element = canvas.current
    if (!element) return
    const context = element.getContext("2d")
    let size: CanvasSize | null = null
    let resolved = resolveColors(element, colors)
    let frame: number | null = null

    const draw = () => {
      frame = null
      if (!context || !size) return
      context.setTransform(size.dpr, 0, 0, size.dpr, 0, 0)
      context.clearRect(0, 0, size.width, size.height)
      painter.current(context, size, resolved)
    }
    request.current = () => {
      frame ??= requestAnimationFrame(draw)
    }
    const stopObserving = observeCanvas(element, (next) => {
      size = next
      // A resized canvas is blank, so it is filled again before it shows.
      if (frame !== null) cancelAnimationFrame(frame)
      draw()
    })
    const stopTheme = subscribeTheme(() => {
      resolved = resolveColors(element, colors)
      request.current()
    })
    return () => {
      stopObserving()
      stopTheme()
      if (frame !== null) cancelAnimationFrame(frame)
      request.current = () => {}
    }
  }, [colors])

  // Every render may have changed what is drawn.
  useEffect(() => {
    painter.current = paint
    request.current()
  })

  return canvas
}
