import type { TickRange } from "@/bindings"
import { xToTick } from "@/lib/canvas"
import { getProjectGeneration } from "@/lib/store/replaced"
import { useProjectStore } from "@/lib/store/project"
import { MAX_SONG_TICKS } from "@/lib/units"
import type { GridMetrics } from "./metrics"
import {
  selectTimelineRegion,
  useTimelineStore,
  type RulerTool,
} from "./timeline-store"

/** A preview-only drag. Cancellation leaves selection, transport and view untouched. */
export class TimelineGesture {
  private readonly generation = getProjectGeneration()
  private readonly revision = useProjectStore.getState().revision
  private readonly viewport
  private readonly from: number
  private cancelled = false
  private readonly metrics: GridMetrics
  private readonly tool: Exclude<RulerTool, "seek">
  constructor(
    metrics: GridMetrics,
    x: number,
    tool: Exclude<RulerTool, "seek">
  ) {
    this.metrics = metrics
    this.tool = tool
    this.viewport = { ...metrics.viewport }
    this.from = this.tick(x)
  }
  private tick(x: number) {
    return Math.max(
      0,
      Math.min(MAX_SONG_TICKS, Math.round(xToTick(this.viewport, x)))
    )
  }
  update(x: number): TickRange | null {
    if (this.cancelled || !Number.isFinite(x)) return null
    const at = this.tick(x)
    const range =
      at === this.from
        ? null
        : { start: Math.min(at, this.from), end: Math.max(at, this.from) }
    useTimelineStore.setState({ draft: range })
    return range
  }
  finish(x: number) {
    if (this.cancelled) return
    const range = this.update(x)
    this.cancel()
    if (
      !range ||
      this.generation !== getProjectGeneration() ||
      this.revision !== useProjectStore.getState().revision
    )
      return
    if (this.tool === "zoom") this.metrics.fitRegion(range)
    else void selectTimelineRegion(range)
  }
  cancel() {
    this.cancelled = true
    useTimelineStore.setState({ draft: null })
  }
}
