import {
  RENDERER_KINDS,
  type RendererChoice,
  type RendererKind,
} from "@/lib/canvas"

export type BenchTheme = "dark" | "light"

export interface BenchParams {
  readonly notes: number
  /** "auto" is what the app will use: WebGL2 on a GPU, else Canvas 2D. */
  readonly renderer: RendererChoice
  readonly theme: BenchTheme
  /** Run the scripted scenarios and publish `window.__benchResults`. */
  readonly auto: boolean
  /** Measured seconds per scenario. */
  readonly seconds: number
  /** Seconds per scenario for the separate GPU timing passes. */
  readonly gpuSeconds: number
  readonly seed: number
  /** Scenario names to run, or null for all. */
  readonly scenarios: readonly string[] | null
  /** Bar numbers and octave names on the overlay. */
  readonly labels: boolean
}

function isRendererKind(value: string): value is RendererKind {
  return RENDERER_KINDS.some((kind) => kind === value)
}

function numberParam(
  search: URLSearchParams,
  name: string,
  fallback: number,
  min: number,
  max: number
): number {
  const raw = search.get(name)
  if (raw === null) return fallback
  const value = Number(raw)
  return Number.isFinite(value) ? Math.min(max, Math.max(min, value)) : fallback
}

export function parseParams(query: string): BenchParams {
  const search = new URLSearchParams(query)
  const renderer = search.get("renderer") ?? ""
  const scenarios = search.get("scenarios")
  return {
    notes: Math.round(numberParam(search, "notes", 10_000, 0, 2_000_000)),
    renderer: isRendererKind(renderer) ? renderer : "auto",
    theme: search.get("theme") === "light" ? "light" : "dark",
    auto: search.get("auto") === "1",
    seconds: numberParam(search, "seconds", 4, 0.2, 60),
    gpuSeconds: numberParam(search, "gpuSeconds", 1.5, 0, 60),
    seed: Math.round(numberParam(search, "seed", 1, 0, 2 ** 31)),
    scenarios: scenarios ? scenarios.split(",").filter((s) => s !== "") : null,
    labels: search.get("labels") !== "0",
  }
}
