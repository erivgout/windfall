import { LATENCY_PRESETS } from "./latency-presets"

export function nextLatencyPreset(
  offset: number,
  direction: "previous" | "next"
): number | null {
  if (!Number.isFinite(offset)) return null

  const index = LATENCY_PRESETS.findIndex(
    (preset) => Math.abs(offset - preset.value) < 0.001
  )
  if (index !== -1) {
    const step = direction === "previous" ? -1 : 1
    return LATENCY_PRESETS[index + step]?.value ?? null
  }

  let previous: number | null = null
  for (const preset of LATENCY_PRESETS) {
    if (direction === "next" && preset.value - offset >= 0.001) return preset.value
    if (offset - preset.value >= 0.001) previous = preset.value
  }
  return direction === "previous" ? previous : null
}
