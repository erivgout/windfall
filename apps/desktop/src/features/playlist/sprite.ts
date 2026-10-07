import type { ClipContent, ClipId, Pattern } from "@/bindings"
import type { DeviceTransform } from "@/lib/canvas"

import type { ClipSpan } from "./preview"

/** The colors one clip is drawn with, as CSS colors. */
export type ClipStyle = {
  body: string
  border: string
  band: string
  bandInk: string
  bodyInk: string
  /** What is drawn inside the clip: its notes, its waveform, its curve. */
  note: string
}

/** A clip as the painter sees it: where it is on the canvas, and what it is. */
export type ClipSprite = {
  /** -1 for a clip that does not exist yet. */
  id: ClipId
  /** Key of the sprite's `ClipStyle`. Sprites are sorted by it. */
  style: number
  content: ClipContent
  name: string
  /** The pattern of a pattern clip, when it still exists. */
  pattern: Pattern | undefined
  /** Where the clip is shown, which a drag moves. */
  span: ClipSpan
  /** The clip's rect in device pixels. */
  x0: number
  x1: number
  y0: number
  y1: number
  ghost: boolean
  selected: boolean
  muted: boolean
}

/** The tick under a device pixel column: the inverse of `deviceX`. */
export function tickAtDeviceX(transform: DeviceTransform, x: number): number {
  return transform.scrollTick + (x + transform.offsetX) / transform.scaleX
}
