// SPDX-License-Identifier: MIT
import { effectivePixelRatio } from "@/lib/ui-scale"
import { backingSize } from "./viewport"

const MAX_EDGE = 8192
const MAX_PIXELS = 16_777_216

/** Keep all layers on one density, within a bounded GPU/2D allocation. */
export function canvasResolution(
  width: number,
  height: number,
  deviceBox?: { readonly inlineSize: number; readonly blockSize: number }
) {
  const w = Number.isFinite(width) ? Math.max(0, width) : 0
  const h = Number.isFinite(height) ? Math.max(0, height) : 0
  const dpr = Math.min(
    effectivePixelRatio(),
    MAX_EDGE / Math.max(1, w, h),
    Math.sqrt(MAX_PIXELS / Math.max(1, w * h))
  )
  const size = backingSize(w, h, dpr, deviceBox)
  const pixelWidth = Math.max(1, Math.min(MAX_EDGE, size.width))
  // Rounding or an observer's exact device box can exceed the area cap by
  // a row. Bound the final allocation as well as its nominal density.
  const pixelHeight = Math.max(
    1,
    Math.min(MAX_EDGE, size.height, Math.floor(MAX_PIXELS / pixelWidth))
  )
  return {
    width: w,
    height: h,
    dpr,
    pixelWidth,
    pixelHeight,
  }
}
