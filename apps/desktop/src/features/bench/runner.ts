import { isSoftwareGpu, type RendererInfo } from "@/lib/canvas"

import type { BenchParams } from "./params"
import type { Scenario, ScenarioContext } from "./scenarios"
import { percentOver, summarize, type Summary } from "./stats"

/** One frame at 60 Hz, the target the plan sets. */
export const FRAME_BUDGET_MS = 16.7
/**
 * A frame interval longer than this many refresh intervals means the
 * browser skipped a display refresh.
 */
export const MISSED_FRAME_FACTOR = 1.5

const WARMUP_MS = 400
const FINISH_PASS_MS = 1000
const SETTLE_FRAMES = 12

export interface GpuSummary extends Summary {
  readonly method: RendererInfo["gpuTiming"]
}

export interface ScenarioResult {
  readonly name: string
  readonly description: string
  readonly frames: number
  /** Frames in which nothing changed and nothing was drawn. */
  readonly idleFrames: number
  readonly seconds: number
  readonly fps: number
  /** Time between animation frames. */
  readonly frameMs: Summary
  /**
   * Percent of frame intervals strictly above 16.7 ms. On a 60 Hz display
   * the interval is 16.67 ms with timestamp jitter on both sides, so this
   * reads high even when no frame is late. `missedFramePct` counts the
   * frames that skipped a refresh.
   */
  readonly overBudgetPct: number
  /** Percent of frame intervals longer than 1.5 display refresh intervals. */
  readonly missedFramePct: number
  /** Main-thread time in the scenario's own update, before drawing. */
  readonly updateCpuMs: Summary
  /** Main-thread time in the draw call (`view.flush`). */
  readonly drawCpuMs: Summary
  /** Average size of the item range sent to the renderer per frame. */
  readonly itemsInRange: number
  /** GPU time per frame from timer queries, in a separate pass. */
  readonly gpuMs: GpuSummary | null
  /** Draw call plus a blocking GPU finish, in a separate pass. WebGL only. */
  readonly drawAndFinishMs: Summary | null
}

export interface BenchMeta {
  readonly userAgent: string
  readonly renderer: RendererInfo
  /** WebGL's unmasked strings, read from a throwaway context. */
  readonly gpuVendor: string
  readonly gpuRenderer: string
  /** True when the GPU string names a software rasterizer. */
  readonly softwareRendering: boolean
  readonly notes: number
  readonly theme: string
  readonly devicePixelRatio: number
  readonly cssWidth: number
  readonly cssHeight: number
  readonly canvasWidth: number
  readonly canvasHeight: number
  /** Median idle frame interval, so the display rate is known. */
  readonly refreshIntervalMs: number
  readonly secondsPerScenario: number
  readonly crossOriginIsolated: boolean
  readonly hardwareConcurrency: number
  readonly visibility: string
  readonly startedAt: string
}

export interface BenchResults {
  readonly meta: BenchMeta
  readonly scenarios: readonly ScenarioResult[]
}

interface PassSamples {
  frameMs: number[]
  updateMs: number[]
  /** Draw time of the frames that drew something. */
  drawMs: number[]
  items: number[]
  gpuMs: number[]
  /** Frames where the scenario changed nothing, so nothing was drawn. */
  idleFrames: number
}

type PassMode = "frames" | "gpu-timer" | "finish"

function nextFrame(): Promise<number> {
  return new Promise((resolve) => requestAnimationFrame(resolve))
}

async function measureRefreshInterval(): Promise<number> {
  const deltas: number[] = []
  let last = await nextFrame()
  for (let i = 0; i < 40; i++) {
    const now = await nextFrame()
    deltas.push(now - last)
    last = now
  }
  return summarize(deltas).p50
}

interface DebugRendererInfo {
  readonly UNMASKED_VENDOR_WEBGL: number
  readonly UNMASKED_RENDERER_WEBGL: number
}

function probeGpu(): { vendor: string; renderer: string } {
  const canvas = document.createElement("canvas")
  const gl = canvas.getContext("webgl2") ?? canvas.getContext("webgl")
  if (!gl) return { vendor: "none", renderer: "no WebGL context" }
  const info: DebugRendererInfo | null = gl.getExtension(
    "WEBGL_debug_renderer_info"
  )
  const vendor: unknown = gl.getParameter(
    info ? info.UNMASKED_VENDOR_WEBGL : gl.VENDOR
  )
  const renderer: unknown = gl.getParameter(
    info ? info.UNMASKED_RENDERER_WEBGL : gl.RENDERER
  )
  gl.getExtension("WEBGL_lose_context")?.loseContext()
  return {
    vendor: typeof vendor === "string" ? vendor : "unknown",
    renderer: typeof renderer === "string" ? renderer : "unknown",
  }
}

/**
 * Runs a scenario's frame function once per animation frame for
 * `durationMs` and records what each frame cost.
 */
function runPass(
  context: ScenarioContext,
  scenario: Scenario,
  durationMs: number,
  mode: PassMode
): Promise<PassSamples> {
  const { view } = context
  const samples: PassSamples = {
    frameMs: [],
    updateMs: [],
    drawMs: [],
    items: [],
    gpuMs: [],
    idleFrames: 0,
  }
  return new Promise((resolve) => {
    let start = -1
    let last = 0
    const step = (timestamp: number): void => {
      if (start < 0) start = timestamp
      else samples.frameMs.push(timestamp - last)
      last = timestamp
      const elapsed = timestamp - start
      if (elapsed >= durationMs) {
        resolve(samples)
        return
      }
      const t0 = performance.now()
      scenario.frame(context, elapsed / durationMs, elapsed)
      const t1 = performance.now()
      const stats = view.flush()
      if (mode === "finish") view.renderer.finish()
      const t2 = performance.now()
      samples.updateMs.push(t1 - t0)
      if (stats) {
        samples.drawMs.push(t2 - t1)
        samples.items.push(stats.itemsInRange)
      } else {
        samples.idleFrames++
      }
      if (mode === "gpu-timer") {
        for (const ms of view.renderer.takeGpuTimes()) samples.gpuMs.push(ms)
      }
      requestAnimationFrame(step)
    }
    requestAnimationFrame(step)
  })
}

async function runScenario(
  context: ScenarioContext,
  scenario: Scenario,
  params: BenchParams,
  refreshIntervalMs: number
): Promise<ScenarioResult> {
  const { view } = context
  const renderer = view.renderer
  scenario.setup(context)
  view.flush()
  await runPass(context, scenario, WARMUP_MS, "frames")

  const measured = await runPass(
    context,
    scenario,
    params.seconds * 1000,
    "frames"
  )

  // Timer queries and finish() both cost something, so they get their own
  // passes and never touch the frame times above.
  let gpuMs: GpuSummary | null = null
  if (params.gpuSeconds > 0 && renderer.setGpuTiming(true)) {
    const pass = await runPass(
      context,
      scenario,
      params.gpuSeconds * 1000,
      "gpu-timer"
    )
    // Query results arrive a few frames late.
    for (let i = 0; i < SETTLE_FRAMES; i++) {
      await nextFrame()
      for (const ms of renderer.takeGpuTimes()) pass.gpuMs.push(ms)
    }
    renderer.setGpuTiming(false)
    if (pass.gpuMs.length > 0) {
      gpuMs = { ...summarize(pass.gpuMs), method: renderer.info.gpuTiming }
    }
  }

  let drawAndFinishMs: Summary | null = null
  if (params.gpuSeconds > 0 && renderer.finish()) {
    const pass = await runPass(context, scenario, FINISH_PASS_MS, "finish")
    drawAndFinishMs = summarize(pass.drawMs)
  }

  scenario.teardown?.(context)
  view.flush()

  const frameMs = summarize(measured.frameMs)
  return {
    name: scenario.name,
    description: scenario.description,
    frames: measured.frameMs.length,
    idleFrames: measured.idleFrames,
    seconds: params.seconds,
    fps: frameMs.avg > 0 ? 1000 / frameMs.avg : 0,
    frameMs,
    overBudgetPct: percentOver(measured.frameMs, FRAME_BUDGET_MS),
    missedFramePct: percentOver(
      measured.frameMs,
      refreshIntervalMs * MISSED_FRAME_FACTOR
    ),
    updateCpuMs: summarize(measured.updateMs),
    drawCpuMs: summarize(measured.drawMs),
    itemsInRange: summarize(measured.items).avg,
    gpuMs,
    drawAndFinishMs,
  }
}

export async function runBenchmark(
  context: ScenarioContext,
  scenarios: readonly Scenario[],
  params: BenchParams,
  onStatus: (status: string) => void
): Promise<BenchResults> {
  const { view } = context
  const startedAt = new Date().toISOString()
  // Let fonts, layout and the first upload settle before timing anything.
  view.flush()
  for (let i = 0; i < 30; i++) await nextFrame()
  const refreshIntervalMs = await measureRefreshInterval()
  const gpu = probeGpu()

  const selected = scenarios.filter(
    (scenario) =>
      params.scenarios === null || params.scenarios.includes(scenario.name)
  )
  const results: ScenarioResult[] = []
  for (const scenario of selected) {
    onStatus(
      `running ${scenario.name} (${results.length + 1}/${selected.length})`
    )
    results.push(
      await runScenario(context, scenario, params, refreshIntervalMs)
    )
  }
  onStatus("done")

  const viewport = view.viewport
  return {
    meta: {
      userAgent: navigator.userAgent,
      renderer: view.renderer.info,
      gpuVendor: gpu.vendor,
      gpuRenderer: gpu.renderer,
      softwareRendering: isSoftwareGpu(gpu.renderer),
      notes: context.model.notes.length,
      theme: params.theme,
      devicePixelRatio: window.devicePixelRatio,
      cssWidth: viewport.width,
      cssHeight: viewport.height,
      canvasWidth: view.element.width,
      canvasHeight: view.element.height,
      refreshIntervalMs,
      secondsPerScenario: params.seconds,
      crossOriginIsolated: window.crossOriginIsolated,
      hardwareConcurrency: navigator.hardwareConcurrency,
      visibility: document.visibilityState,
      startedAt,
    },
    scenarios: results,
  }
}
