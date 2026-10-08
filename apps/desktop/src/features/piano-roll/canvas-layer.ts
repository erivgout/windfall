import { observePixelRatio } from "@/lib/ui-scale"
import { canvasResolution } from "@/lib/canvas/resolution"

export type LayerSize = {
  /** Backing size in device pixels. */
  width: number
  height: number
  /** This layer's bounded density; it can differ from the grid's density.
   * Painters must derive device transforms from this value, not view.transform.
   */
  dpr: number
}

export type LayerPainter = (
  ctx: CanvasRenderingContext2D,
  size: LayerSize
) => void

/**
 * A small 2D canvas beside the note grid: the ruler and the value lane.
 * It keeps its backing store at device resolution and draws at most once
 * per animation frame, however often it is invalidated.
 */
export class CanvasLayer {
  private readonly canvas: HTMLCanvasElement
  private readonly ctx: CanvasRenderingContext2D | null
  private readonly paint: LayerPainter
  private readonly observer: ResizeObserver
  private readonly stopPixelObserver: () => void
  private frame = 0
  private destroyed = false
  private size: ReturnType<typeof canvasResolution> | null = null

  constructor(canvas: HTMLCanvasElement, paint: LayerPainter) {
    this.canvas = canvas
    this.ctx = canvas.getContext("2d")
    this.paint = paint
    this.observer = new ResizeObserver(() => {
      if (this.resize()) this.flush()
    })
    this.observer.observe(canvas)
    this.stopPixelObserver = observePixelRatio(() => {
      this.resize()
      this.invalidate()
    })
    this.resize()
    this.invalidate()
  }

  invalidate(): void {
    if (this.destroyed || this.frame !== 0) return
    this.frame = requestAnimationFrame(() => {
      this.frame = 0
      this.flush()
    })
  }

  destroy(): void {
    this.destroyed = true
    this.observer.disconnect()
    this.stopPixelObserver()
    if (this.frame !== 0) cancelAnimationFrame(this.frame)
  }

  private resize(): boolean {
    const next = canvasResolution(
      this.canvas.clientWidth,
      this.canvas.clientHeight
    )
    const previous = this.size
    this.size = next
    if (this.canvas.width !== next.pixelWidth)
      this.canvas.width = next.pixelWidth
    if (this.canvas.height !== next.pixelHeight)
      this.canvas.height = next.pixelHeight
    // A capped store can keep the same pixel dimensions while its logical
    // size/density changes. The painter still needs the new transform.
    return (
      !previous ||
      previous.width !== next.width ||
      previous.height !== next.height ||
      previous.dpr !== next.dpr ||
      previous.pixelWidth !== next.pixelWidth ||
      previous.pixelHeight !== next.pixelHeight
    )
  }

  // Resizing clears a canvas, so it is drawn again before the browser paints.
  private flush(): void {
    const ctx = this.ctx
    if (!ctx || this.destroyed) return
    this.resize()
    const { width, height } = this.canvas
    ctx.setTransform(1, 0, 0, 1, 0, 0)
    ctx.clearRect(0, 0, width, height)
    ctx.save()
    this.paint(ctx, {
      width,
      height,
      dpr: this.size!.dpr,
    })
    ctx.restore()
  }
}
