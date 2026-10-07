import type { RectBatch } from "./rect-batch"
import type { GridTheme } from "./theme"
import type { DeviceTransform } from "./viewport"

export type RendererKind = "canvas2d" | "webgl2" | "webgpu"

export const RENDERER_KINDS: readonly RendererKind[] = [
  "canvas2d",
  "webgl2",
  "webgpu",
]

/** Border color of an item as a fraction of its fill color. */
export const BORDER_SHADE = 0.62

export interface RendererInfo {
  readonly kind: RendererKind
  /** GPU or driver name when the API exposes one. */
  readonly device: string
  /** How `takeGpuTimes` measures, or "none" when it cannot. */
  readonly gpuTiming: "timer-query" | "timestamp-query" | "none"
}

export interface DrawOptions {
  /** Index range to draw, `[first, last)`. Defaults to the whole batch. */
  readonly first?: number
  readonly last?: number
  /** Offset applied to selected rects while they are being dragged. */
  readonly dragTicks?: number
  readonly dragRows?: number
  /**
   * Ticks added to the start edge of selected rects while they are being
   * resized. The end edge stays where it is. See `resizedSpan`.
   */
  readonly resizeStart?: number
  /** Ticks added to the end edge of selected rects. */
  readonly resizeEnd?: number
  /** A resize never leaves a rect shorter than this, or than it was. */
  readonly minLength?: number
}

/**
 * Start and length of a rect after a resize drag. Every renderer applies
 * exactly this to selected rects, so commit a resize with it and the drop
 * lands where the preview was. With both deltas 0 it changes nothing.
 */
export function resizedSpan(
  start: number,
  length: number,
  resizeStart: number,
  resizeEnd: number,
  minLength: number
): { start: number; length: number } {
  const keep = Math.min(length, minLength)
  const shift = Math.min(resizeStart, length - keep)
  const shrunk = length - shift
  return {
    start: start + shift,
    length: Math.max(shrunk + resizeEnd, Math.min(shrunk, minLength)),
  }
}

/**
 * Draws rect batches into one canvas. A frame is `beginFrame`, any number
 * of `drawBatch` calls back to front, then `endFrame`.
 *
 * A batch is uploaded when its version counters change, so drawing the
 * same batch again with a new transform or drag offset moves no data.
 * Within a batch, selected rects are drawn after the others.
 */
export interface RectRenderer {
  readonly info: RendererInfo
  resize(widthDev: number, heightDev: number): void
  setTheme(theme: GridTheme): void
  beginFrame(transform: DeviceTransform): void
  drawBatch(batch: RectBatch, options?: DrawOptions): void
  endFrame(): void
  /** Frees whatever the renderer holds for a batch that will not be drawn again. */
  release(batch: RectBatch): void
  /**
   * Blocks until the GPU has finished the submitted work, where the API
   * can. Returns false when it cannot. For measurement only.
   */
  finish(): boolean
  /** Turns per-frame GPU timing on or off. Returns whether it is supported. */
  setGpuTiming(enabled: boolean): boolean
  /** GPU milliseconds of frames finished since the last call. */
  takeGpuTimes(): number[]
  /** Called when the drawing surface was lost and restored, so the owner can redraw. */
  onRestored: (() => void) | null
  dispose(): void
}

/**
 * True when a GPU name belongs to a CPU rasterizer (SwiftShader, llvmpipe,
 * WARP). Instanced WebGL on one of these is far slower than Canvas 2D.
 */
export function isSoftwareGpu(device: string): boolean {
  return /swiftshader|llvmpipe|softpipe|software|basic render/i.test(device)
}

export class RendererUnavailableError extends Error {
  readonly kind: RendererKind

  constructor(kind: RendererKind, reason: string) {
    super(`${kind} renderer unavailable: ${reason}`)
    this.name = "RendererUnavailableError"
    this.kind = kind
  }
}
