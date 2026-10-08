// SPDX-License-Identifier: MIT
import { logicalDelta, observePixelRatio } from "@/lib/ui-scale"
import { canvasResolution } from "@/lib/canvas/resolution"

/**
 * Resolves CSS color expressions (`var(--wf-waveform)`, `color-mix(...)`) to
 * colors a canvas can paint with, as seen from `element`, so variables
 * overridden on an ancestor are honored.
 */
export function resolveColors<Name extends string>(
  element: Element,
  colors: Record<Name, string>
): Record<Name, string> {
  const probe = document.createElement("span")
  probe.style.display = "none"
  element.appendChild(probe)
  const resolved = {} as Record<Name, string>
  for (const name of Object.keys(colors) as Name[]) {
    probe.style.color = ""
    probe.style.color = colors[name]
    resolved[name] = getComputedStyle(probe).color || "transparent"
  }
  probe.remove()
  return resolved
}

const themeListeners = new Set<() => void>()
let themeObserver: MutationObserver | null = null
let schemeQuery: MediaQueryList | null = null

function notifyTheme() {
  for (const listener of themeListeners) {
    listener()
  }
}

/**
 * Calls `listener` when the page theme may have changed: the class, style or
 * data-theme of `<html>` or `<body>`, or the system color scheme. Canvas
 * components use it to read their colors again.
 */
export function subscribeTheme(listener: () => void): () => void {
  if (themeListeners.size === 0) {
    const attributeFilter = ["class", "style", "data-theme"]
    themeObserver = new MutationObserver(notifyTheme)
    themeObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter,
    })
    themeObserver.observe(document.body, { attributes: true, attributeFilter })
    schemeQuery = window.matchMedia?.("(prefers-color-scheme: dark)") ?? null
    schemeQuery?.addEventListener("change", notifyTheme)
  }
  themeListeners.add(listener)
  return () => {
    themeListeners.delete(listener)
    if (themeListeners.size === 0) {
      themeObserver?.disconnect()
      themeObserver = null
      schemeQuery?.removeEventListener("change", notifyTheme)
      schemeQuery = null
    }
  }
}

export type CanvasSize = {
  /** Size in CSS pixels. */
  width: number
  height: number
  /** Size of the backing store in device pixels. */
  pixelWidth: number
  pixelHeight: number
  dpr: number
}

/**
 * Keeps a canvas's backing store matched to its CSS size and the device
 * pixel ratio. `onChange` runs once at the start and after every resize or
 * ratio change (moving the window to another monitor, browser zoom).
 */
export function observeCanvas(
  canvas: HTMLCanvasElement,
  onChange: (size: CanvasSize) => void
): () => void {
  const measure = () => {
    const rect = canvas.getBoundingClientRect()
    const size = canvasResolution(
      logicalDelta(rect.width),
      logicalDelta(rect.height)
    )
    const { pixelWidth, pixelHeight } = size
    if (canvas.width !== pixelWidth) {
      canvas.width = pixelWidth
    }
    if (canvas.height !== pixelHeight) {
      canvas.height = pixelHeight
    }
    onChange(size)
  }

  const resizeObserver =
    typeof ResizeObserver === "undefined" ? null : new ResizeObserver(measure)
  resizeObserver?.observe(canvas)
  const stopPixelObserver = observePixelRatio(measure)

  return () => {
    resizeObserver?.disconnect()
    stopPixelObserver()
  }
}
