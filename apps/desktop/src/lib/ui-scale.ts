// SPDX-License-Identifier: MIT

/** Display preferences and coordinate conversions; no document/backend state. */
export const UI_SCALES = [75, 100, 125, 150, 175, 200] as const
export type UiScale = (typeof UI_SCALES)[number]
export const DEFAULT_UI_SCALE: UiScale = 100
export const UI_SCALE_EVENT = "windfall:ui-scale"

export function supportsUiScale(): boolean {
  return (
    typeof CSS === "undefined" ||
    typeof CSS.supports !== "function" ||
    CSS.supports("zoom", "1")
  )
}

export function validUiScale(value: unknown): UiScale {
  return UI_SCALES.includes(value as UiScale)
    ? (value as UiScale)
    : DEFAULT_UI_SCALE
}

/** Application scale only. Browser/monitor zoom is already in client pixels. */
export function uiScaleFactor(): number {
  if (typeof document === "undefined") return 1
  return validUiScale(Number(document.documentElement.dataset.uiScale)) / 100
}

export function logicalDelta(pixels: number): number {
  return pixels / uiScaleFactor()
}

export function localPoint(
  element: Element,
  event: { clientX: number; clientY: number }
) {
  const bounds = element.getBoundingClientRect()
  return {
    x: logicalDelta(event.clientX - bounds.left) - element.clientLeft,
    y: logicalDelta(event.clientY - bounds.top) - element.clientTop,
  }
}

/** Line/page units are logical; pixel wheel units are visual client pixels. */
export function logicalWheel(
  event: Pick<WheelEvent, "deltaX" | "deltaY" | "deltaMode">,
  units: { line: number; page: number } = { line: 16, page: 400 }
) {
  const unit =
    event.deltaMode === 1
      ? units.line
      : event.deltaMode === 2
        ? units.page
        : 1 / uiScaleFactor()
  return { deltaX: event.deltaX * unit, deltaY: event.deltaY * unit }
}

export function effectivePixelRatio(): number {
  const dpr = window.devicePixelRatio
  return (Number.isFinite(dpr) && dpr > 0 ? dpr : 1) * uiScaleFactor()
}

/** ResizeObserver need not fire when zoom changes without a layout resize. */
export function observePixelRatio(listener: () => void): () => void {
  let query: MediaQueryList | undefined
  function changed() {
    query?.removeEventListener("change", changed)
    query = window.matchMedia?.(`(resolution: ${window.devicePixelRatio}dppx)`)
    query?.addEventListener("change", changed)
    listener()
  }
  changed()
  window.addEventListener(UI_SCALE_EVENT, changed)
  window.addEventListener("resize", changed)
  return () => {
    query?.removeEventListener("change", changed)
    window.removeEventListener(UI_SCALE_EVENT, changed)
    window.removeEventListener("resize", changed)
  }
}

/** Root zoom includes body portals and lets CSS layout/scrollbars reflow. */
export function applyUiScale(value: unknown): void {
  const scale = supportsUiScale() ? validUiScale(value) : DEFAULT_UI_SCALE
  const root = document.documentElement
  root.dataset.uiScale = String(scale)
  root.style.setProperty("--wf-ui-scale", String(scale / 100))
  root.style.zoom = String(scale / 100)
  root.style.setProperty(
    "--wf-viewport-width",
    `${window.innerWidth / (scale / 100)}px`
  )
  root.style.setProperty(
    "--wf-viewport-height",
    `${window.innerHeight / (scale / 100)}px`
  )
  window.dispatchEvent(new Event(UI_SCALE_EVENT))
}
