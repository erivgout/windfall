export type LayerSize = {
  /** Backing size in device pixels. */
  width: number
  height: number
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
  private frame = 0
  private destroyed = false

  constructor(canvas: HTMLCanvasElement, paint: LayerPainter) {
    this.canvas = canvas
    this.ctx = canvas.getContext("2d")
    this.paint = paint
    this.observer = new ResizeObserver(() => {
      if (this.resize()) this.flush()
    })
    this.observer.observe(canvas)
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
    if (this.frame !== 0) cancelAnimationFrame(this.frame)
  }

  private resize(): boolean {
    const dpr = window.devicePixelRatio || 1
    const width = Math.max(1, Math.round(this.canvas.clientWidth * dpr))
    const height = Math.max(1, Math.round(this.canvas.clientHeight * dpr))
    if (this.canvas.width === width && this.canvas.height === height) {
      return false
    }
    this.canvas.width = width
    this.canvas.height = height
    return true
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
    this.paint(ctx, { width, height, dpr: window.devicePixelRatio || 1 })
    ctx.restore()
  }
}
