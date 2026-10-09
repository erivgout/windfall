import { LOOP_REGION_PRESETS, nextLoopRegion } from "./loop-region-presets"

export function nextLoopRegionPreset(
  start: number,
  end: number,
  direction: "previous" | "next"
): { start: number; end: number } | null {
  const index = LOOP_REGION_PRESETS.findIndex(
    (preset) => nextLoopRegion(start, end, preset) === null
  )
  if (index === -1) return null

  const step = direction === "previous" ? -1 : 1
  const preset = LOOP_REGION_PRESETS[index + step]
  return preset ? { start: preset.start, end: preset.end } : null
}
